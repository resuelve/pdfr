use std::collections::BTreeMap;

use lopdf::{Bookmark, Dictionary, Document, Object, ObjectId};
use rustler::{Binary, Env, OwnedBinary};

type DocumentBinaryData = Vec<u8>;

/// Entries a page inherits from its ancestors in the page tree when it doesn't define
/// them itself (PDF 32000-1:2008, 7.7.3.4). The merged document keeps a single /Pages
/// node, so they are copied onto each page before the original nodes are dropped.
const INHERITABLE_ENTRIES: [&[u8]; 4] = [b"Resources", b"MediaBox", b"CropBox", b"Rotate"];

enum SaveMethod<'a> {
    Buffer(&'a mut DocumentBinaryData),
    File(&'a str),
}

// Both NIFs are CPU bound and unbounded in the size of their input: merging one Mexican
// contract (19 documents, 20 MB) takes hundreds of milliseconds even after this pass, and a
// normal NIF holds its scheduler for the whole call. Measured before moving them off: a
// 163 ms merge froze an unrelated BEAM process for 127 ms.
#[rustler::nif(schedule = "DirtyCpu")]
fn merge(pdfs_to_merge: Vec<&str>, merged_pdf_name: &str) -> Result<(), String> {
    let documents = pdfs_to_merge
        .iter()
        .map(|path| Document::load(path).map_err(|error| format!("{}: {}", path, error)))
        .collect::<Result<Vec<Document>, String>>()?;

    run_merge(documents, SaveMethod::File(merged_pdf_name))
}

#[rustler::nif(schedule = "DirtyCpu")]
fn merge_bin<'a>(env: Env<'a>, pdfs_to_merge: Vec<Binary<'a>>) -> Result<Binary<'a>, String> {
    // The documents arrive as binaries and leave as one binary: going through Erlang lists
    // meant a term per byte in both directions, ~3.5 s and ~800 MB of heap per contract
    let documents = pdfs_to_merge
        .iter()
        .enumerate()
        .map(|(index, document)| {
            Document::load_mem(document.as_slice())
                .map_err(|error| format!("Document {}: {}", index, error))
        })
        .collect::<Result<Vec<Document>, String>>()?;

    let mut buffer: DocumentBinaryData = vec![];
    run_merge(documents, SaveMethod::Buffer(&mut buffer))?;

    let mut merged = OwnedBinary::new(buffer.len()).ok_or_else(|| {
        format!(
            "Could not allocate {} bytes for the merged PDF",
            buffer.len()
        )
    })?;
    merged.as_mut_slice().copy_from_slice(&buffer);

    Ok(merged.release(env))
}

/// Value of `key` for a page, looked up the /Parent chain when the page itself doesn't
/// define it. Bounded so a malformed /Parent cycle can't loop forever.
fn inherited_entry(doc: &Document, page_id: ObjectId, key: &[u8]) -> Option<Object> {
    let mut node_id = page_id;

    for _ in 0..64 {
        let node = doc.get_object(node_id).ok()?.as_dict().ok()?;

        if let Ok(value) = node.get(key) {
            return Some(value.clone());
        }

        node_id = node.get(b"Parent").ok()?.as_reference().ok()?;
    }

    None
}

fn run_merge(documents: Vec<Document>, save_method: SaveMethod) -> Result<(), String> {
    // Define a starting max_id (will be used as start index for object_ids)
    let mut max_id = 1;
    let mut pagenum = 1;
    // Collect all Documents Objects grouped by a map
    let mut documents_pages = BTreeMap::new();
    let mut documents_objects = BTreeMap::new();
    let mut document = Document::with_version("1.4");

    for mut doc in documents {
        let mut first = false;
        doc.renumber_objects_with(max_id);

        max_id = doc.max_id + 1;

        for (_, object_id) in doc.get_pages() {
            if !first {
                let bookmark =
                    Bookmark::new(format!("Page_{}", pagenum), [0.0, 0.0, 1.0], 0, object_id);
                document.add_bookmark(bookmark, None);
                first = true;
                pagenum += 1;
            }

            let mut page = match doc.get_object(object_id).and_then(Object::as_dict) {
                Ok(dictionary) => dictionary.clone(),
                Err(_) => continue,
            };

            // The page is about to lose its ancestors, so whatever it inherited from them
            // has to travel with it
            for &key in INHERITABLE_ENTRIES.iter() {
                if !page.has(key) {
                    if let Some(value) = inherited_entry(&doc, object_id, key) {
                        page.set(key, value);
                    }
                }
            }

            documents_pages.insert(object_id, Object::Dictionary(page));
        }

        documents_objects.extend(doc.objects);
    }

    if documents_pages.is_empty() {
        return Err("No pages found in the documents to merge".to_string());
    }

    // A fresh root for the page tree, with an id above every renumbered object
    let pages_root_id: ObjectId = (max_id, 0);

    // Catalog is mandatory
    let mut catalog_object: Option<(ObjectId, Object)> = None;

    // Process all objects except "Page" and "Pages" types.
    // Consumed by value: the inputs are dropped right after this, and cloning every object of
    // a 20 MB contract duplicated the whole content stream set for nothing
    for (object_id, object) in documents_objects {
        // We have to ignore "Page" (as are processed later), "Pages", "Outlines" and "Outline"
        // objects. All other objects should be collected and inserted into the main Document

        let type_name = object.type_name().unwrap_or("");

        match type_name {
            "Catalog" => {
                // Collect a first "Catalog" object and use it for the future "Pages"
                catalog_object = Some((
                    if let Some((id, _)) = catalog_object {
                        id
                    } else {
                        object_id
                    },
                    object,
                ));
            }
            // Every /Pages node of the inputs is dropped, the root of each document and the
            // intermediate nodes alike, and the pages are re-parented to the fresh root
            // built below. Folding their dictionaries into one, as the lopdf merge example
            // did, turned the /Parent of the intermediate nodes into an array on the merged
            // root (qpdf: "root of pages tree has no /Kids array") and would do the same to
            // any inheritable entry they carried
            "Pages" => {}
            "Page" => {}     // Ignored, processed later and separately
            "Outlines" => {} // Ignored, not supported yet
            "Outline" => {}  // Ignored, not supported yet
            _ => {
                document.objects.insert(object_id, object);
            }
        }
    }

    // If no "Catalog" found abort
    let catalog_object = match catalog_object {
        Some(catalog_object) => catalog_object,
        None => return Err("Catalog root not found".to_string()),
    };

    // Iter over all "Page" and hang them from the new root
    for (object_id, object) in documents_pages.iter() {
        if let Ok(dictionary) = object.as_dict() {
            let mut dictionary = dictionary.clone();
            dictionary.set("Parent", pages_root_id);

            document
                .objects
                .insert(*object_id, Object::Dictionary(dictionary));
        }
    }

    // Build the new "Pages" root: only the pages, in document order
    let mut pages_root = Dictionary::new();
    pages_root.set("Type", Object::Name(b"Pages".to_vec()));
    pages_root.set("Count", documents_pages.len() as u32);
    pages_root.set(
        "Kids",
        documents_pages
            .keys()
            .map(|object_id| Object::Reference(*object_id))
            .collect::<Vec<_>>(),
    );

    document
        .objects
        .insert(pages_root_id, Object::Dictionary(pages_root));

    // Build a new "Catalog" with updated fields
    if let Ok(dictionary) = catalog_object.1.as_dict() {
        let mut dictionary = dictionary.clone();
        dictionary.set("Pages", pages_root_id);
        dictionary.remove(b"Outlines"); // Outlines not supported in merged PDFs

        document
            .objects
            .insert(catalog_object.0, Object::Dictionary(dictionary));
    }

    document.trailer.set("Root", catalog_object.0);

    // Update the max internal ID as wasn't updated before due to direct objects insertion.
    // It has to be the highest id actually present, not `objects.len()`: the ids are unique
    // (each input was renumbered above a running counter) but sparse, because every /Pages
    // node of the inputs was dropped. lopdf writes the xref table as `1..max_id + 1`, so an
    // id past `max_id` is written into the file yet left out of the table — present on disk
    // and unreachable on the next read. Renumbering into 1..n instead, as this did before,
    // hid the gap at a cost of ~0.9 s per contract and bought nothing else
    document.max_id = document
        .objects
        .keys()
        .map(|(id, _)| *id)
        .max()
        .unwrap_or(0);

    //Set any Bookmarks to the First child if they are not set to a page
    document.adjust_zero_pages();

    //Set all bookmarks to the PDF Object tree then set the Outlines to the Bookmark content map.
    if let Some(n) = document.build_outline() {
        if let Ok(x) = document.get_object_mut(catalog_object.0) {
            if let Object::Dictionary(ref mut dict) = x {
                dict.set("Outlines", Object::Reference(n));
            }
        }
    }

    document.compress();

    // Save the merged PDF. A write that fails has to travel back as an error: swallowing it
    // handed the caller a truncated buffer as {:ok, _}, and panicking took the NIF with it
    match save_method {
        SaveMethod::Buffer(buffer) => document
            .save_to(buffer)
            .map_err(|error| format!("Could not write the merged PDF: {}", error))?,
        SaveMethod::File(merged_pdf_name) => document
            .save(merged_pdf_name)
            .map(|_| ())
            .map_err(|error| format!("Could not save {}: {}", merged_pdf_name, error))?,
    }

    Ok(())
}

rustler::init!("Elixir.Pdfr.Pdf", [merge, merge_bin]);

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::Stream;

    fn name(value: &str) -> Object {
        Object::Name(value.as_bytes().to_vec())
    }

    fn media_box(width: i64, height: i64) -> Object {
        Object::Array(vec![
            Object::Integer(0),
            Object::Integer(0),
            Object::Integer(width),
            Object::Integer(height),
        ])
    }

    fn page(doc: &mut Document, parent: ObjectId, media_box: Option<Object>) -> ObjectId {
        let contents = doc.add_object(Stream::new(Dictionary::new(), b"".to_vec()));
        let mut page = Dictionary::new();
        page.set("Type", name("Page"));
        page.set("Parent", parent);
        page.set("Contents", contents);
        if let Some(media_box) = media_box {
            page.set("MediaBox", media_box);
        }
        doc.add_object(page)
    }

    fn finish(doc: &mut Document, pages_root: ObjectId, kids: Vec<ObjectId>, count: u32) {
        let mut root = Dictionary::new();
        root.set("Type", name("Pages"));
        root.set("Count", count);
        root.set(
            "Kids",
            kids.into_iter().map(Object::Reference).collect::<Vec<_>>(),
        );
        doc.objects.insert(pages_root, Object::Dictionary(root));

        let mut catalog = Dictionary::new();
        catalog.set("Type", name("Catalog"));
        catalog.set("Pages", pages_root);
        let catalog_id = doc.add_object(catalog);
        doc.trailer.set("Root", catalog_id);
    }

    /// Two pages straight under the root, each with its own MediaBox: what pypdf writes
    fn flat_document() -> Document {
        let mut doc = Document::with_version("1.4");
        let pages_root = doc.new_object_id();
        let kids = vec![
            page(&mut doc, pages_root, Some(media_box(612, 792))),
            page(&mut doc, pages_root, Some(media_box(612, 792))),
        ];
        finish(&mut doc, pages_root, kids, 2);
        doc
    }

    /// Three pages, the first two under an intermediate node that owns the MediaBox they
    /// inherit: the shape of Word and Google Docs exports, which qpdf preserves
    fn nested_document() -> Document {
        let mut doc = Document::with_version("1.4");
        let pages_root = doc.new_object_id();
        let intermediate = doc.new_object_id();

        let under_intermediate = vec![
            page(&mut doc, intermediate, None),
            page(&mut doc, intermediate, None),
        ];
        let mut node = Dictionary::new();
        node.set("Type", name("Pages"));
        node.set("Parent", pages_root);
        node.set("Count", 2u32);
        node.set("MediaBox", media_box(595, 842));
        node.set(
            "Kids",
            under_intermediate
                .into_iter()
                .map(Object::Reference)
                .collect::<Vec<_>>(),
        );
        doc.objects.insert(intermediate, Object::Dictionary(node));

        let third = page(&mut doc, pages_root, Some(media_box(612, 792)));
        finish(&mut doc, pages_root, vec![intermediate, third], 3);
        doc
    }

    fn merge_into_memory(documents: Vec<Document>) -> Document {
        let mut buffer = vec![];
        run_merge(documents, SaveMethod::Buffer(&mut buffer)).unwrap();
        Document::load_mem(&buffer).unwrap()
    }

    #[test]
    fn the_merged_document_has_a_single_flat_page_tree() {
        let merged = merge_into_memory(vec![nested_document(), flat_document()]);

        let root_id = merged
            .catalog()
            .unwrap()
            .get(b"Pages")
            .unwrap()
            .as_reference()
            .unwrap();
        let root = merged.get_object(root_id).unwrap().as_dict().unwrap();

        // the defect: a /Parent array on the root, inherited from the intermediate nodes
        assert!(!root.has(b"Parent"));
        assert_eq!(root.get(b"Count").unwrap().as_i64().unwrap(), 5);

        let kids = root.get(b"Kids").unwrap().as_array().unwrap();
        assert_eq!(kids.len(), 5);
        for kid in kids {
            let kid = merged
                .get_object(kid.as_reference().unwrap())
                .unwrap()
                .as_dict()
                .unwrap();
            assert_eq!(kid.get(b"Type").unwrap().as_name().unwrap(), b"Page");
            assert_eq!(kid.get(b"Parent").unwrap().as_reference().unwrap(), root_id);
        }

        assert_eq!(merged.get_pages().len(), 5);
    }

    #[test]
    fn pages_keep_what_they_inherited_from_the_dropped_nodes() {
        let merged = merge_into_memory(vec![nested_document(), flat_document()]);

        let widths: Vec<i64> = merged
            .get_pages()
            .values()
            .map(|page_id| {
                let page = merged.get_object(*page_id).unwrap().as_dict().unwrap();
                page.get(b"MediaBox").unwrap().as_array().unwrap()[2]
                    .as_i64()
                    .unwrap()
            })
            .collect();

        assert_eq!(widths, vec![595, 595, 612, 612, 612]);
    }

    /// Guard for the id space now that the final `renumber_objects()` is gone: the writer
    /// builds the xref as `1..max_id + 1`, so an object numbered past `max_id` never makes it
    /// into the table and the next read simply doesn't see it. With `max_id = objects.len()`
    /// — the ids are sparse — the pages survive but the streams they point at are dropped.
    #[test]
    fn every_object_of_the_merged_document_fits_in_the_declared_id_space() {
        let merged = merge_into_memory(vec![nested_document(), flat_document()]);

        let highest = merged.objects.keys().map(|(id, _)| *id).max().unwrap();
        assert!(
            merged.max_id >= highest,
            "the xref declares {} ids but the document reaches {}",
            merged.max_id,
            highest
        );

        // and everything the page tree hangs off has to be reachable through it
        assert_eq!(merged.get_pages().len(), 5);

        for (number, page_id) in merged.get_pages() {
            let page = merged.get_object(page_id).unwrap().as_dict().unwrap();
            let contents = page.get(b"Contents").unwrap().as_reference().unwrap();

            assert!(
                merged.get_object(contents).is_ok(),
                "page {} lost its contents {:?}",
                number,
                contents
            );
        }
    }

    #[test]
    fn merging_nothing_with_pages_is_an_error() {
        let mut doc = Document::with_version("1.4");
        let pages_root = doc.new_object_id();
        finish(&mut doc, pages_root, vec![], 0);

        let mut buffer = vec![];
        assert!(run_merge(vec![doc], SaveMethod::Buffer(&mut buffer)).is_err());
    }
}
