# Pdfr

MVP for a PDF managment library using Rust NIFs

> This is an MVP, DO NOT USE

## Functionality

### PDF Merge

You can merge all specified files into a single PDF
```elixir
Pdfr.merge(["/some/file", "some/other/file"], "merged.pdf")
```

or you can merge all `.pdf` files into one
``` elixir
Pdfr.merge_all("some/dir/", "merged.pdf")
```

or, without touching the filesystem, you can merge documents you already hold in memory.
Every document is iodata — a binary, or an iolist that gets flattened before the call — and
the merged PDF comes back as a binary:

``` elixir
{:ok, merged} = Pdfr.merge_bin([File.read!("a.pdf"), File.read!("b.pdf")])
```

A malformed document comes back as `{:error, reason}`; it does not raise.

### Scheduling

Both NIFs run on a **dirty CPU scheduler**. The work is CPU bound and unbounded in the size
of its input — parsing, merging and writing a 20 MB set still takes hundreds of milliseconds —
and a normal NIF would hold a regular scheduler for the whole call, stalling every other
process on it.

## Installation

If [available in Hex](https://hex.pm/docs/publish), the package can be installed
by adding `pdfr` to your list of dependencies in `mix.exs`:

```elixir
def deps do
  [
    {:pdfr, "~> 0.1.0"}
  ]
end
```

Documentation can be generated with [ExDoc](https://github.com/elixir-lang/ex_doc)
and published on [HexDocs](https://hexdocs.pm). Once published, the docs can
be found at <https://hexdocs.pm/pdfr>.

