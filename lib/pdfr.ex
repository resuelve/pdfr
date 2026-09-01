defmodule Pdfr do
  @moduledoc """
  Documentation for `Pdfr`.
  """

  alias Pdfr.Pdf

  @doc """
  Merges all specified PDF files into a single PDF
  """
  @spec merge([String.t()], String.t()) :: :ok | {:error, String.t()}
  def merge([_ | _] = pdfs_to_merge, merged_pdf_name) do
    case Pdf.merge(pdfs_to_merge, merged_pdf_name) do
      {:ok, _} ->
        :ok

      {:error, error} ->
        {:error, error}
    end
  end

  @doc """
  Merges all PDF files from the specfied directory into a single PDF
  """
  @spec merge_all(String.t(), String.t()) :: :ok | {:error, String.t()}
  def merge_all(dir_path, merged_pdf_name) do
    pdfs_to_merge =
      dir_path
      |> File.ls!()
      |> Enum.filter(&String.ends_with?(&1, ".pdf"))
      |> Enum.map(&Path.join(dir_path, &1))

    merge(pdfs_to_merge, merged_pdf_name)
  end

  @doc """
  Merges all PDF iodata given into a single file

  Every document is iodata: a binary, or an iolist that gets flattened here. The merged
  PDF comes back as a binary.
  """
  @spec merge_bin([iodata()]) :: {:ok, binary()} | {:error, String.t()}
  def merge_bin(documents) do
    # The NIF takes and returns binaries. An iolist is flattened once, here; travelling to
    # Rust as a list of bytes and coming back as another one cost ~3.5 s and ~800 MB of heap
    # for a 20 MB contract
    documents =
      Enum.map(documents, fn
        document when is_binary(document) -> document
        [_ | _] = document -> IO.iodata_to_binary(document)
        _ -> raise "Documents should be of iodata type"
      end)

    Pdf.merge_bin(documents)
  end
end
