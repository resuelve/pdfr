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
  """
  @spec merge_bin([iodata()]) :: :ok | {:error, String.t()}
  def merge_bin(documents) do
    merged_name = "prueba_bin.pdf"

    case Pdf.merge_bin(documents, merged_name) do
      {:ok, _} ->
        :ok

      {:error, error} ->
        {:error, error}
    end
  end
end
