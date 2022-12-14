defmodule Pdfr do
  @moduledoc """
  Documentation for `Pdfr`.
  """

  alias Pdfr.Pdf

  @typep nif_response :: {:ok, any()} | {:error, String.t()}

  @doc """
  Merges all specified PDF files into a single PDF
  """
  @spec merge([String.t()], String.t()) :: nif_response
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
  @spec merge_all(String.t(), String.t()) :: nif_response
  def merge_all(dir_path, merged_pdf_name) do
    pdfs_to_merge =
      dir_path
      |> File.ls!()
      |> Enum.filter(&String.ends_with?(&1, ".pdf"))
      |> Enum.map(&Path.join(dir_path, &1))

    merge(pdfs_to_merge, merged_pdf_name)
  end
end
