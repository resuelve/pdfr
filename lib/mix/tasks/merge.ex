defmodule Mix.Tasks.Merge do
  @moduledoc "Printed when the user requests `mix help echo`"
  @shortdoc "Echoes arguments"

  use Mix.Task

  @impl Mix.Task

  def run(["all", dir_path, merged_pdf_name | _]) do
    Mix.shell().info("Merging all docs in #{dir_path} as #{merged_pdf_name}")

    case Pdfr.merge_all(dir_path, merged_pdf_name) do
      :ok ->
        Mix.shell().info("PDF merged as #{merged_pdf_name} succesfuly")

      error ->
        Mix.shell().error(inspect(error))
    end
  end

  def run(["bin" | _]) do
    pdfs =
      [
        "/Users/poncho/dev/test-files/pdf1_72.pdf",
        "/Users/poncho/dev/test-files/pdf1_72.pdf"
      ]
      |> Enum.map(fn p ->
        f = File.open!(p)
        data = IO.binread(f, :all)
        File.close(f)

        :binary.bin_to_list(data)
      end)

    case Pdfr.merge_bin(pdfs) do
      {:ok, bin_data} ->
        f = File.open!("mergebin.pdf", [:write])
        IO.binwrite(f, bin_data)
        File.close(f)
        Mix.shell().info("PDF merged succesfuly")

      error ->
        Mix.shell().error(inspect(error))
    end
  end

  def run([merged_pdf_name | _]) do
    pdfs = [
      "/Users/poncho/dev/test-files/pdf1_72.pdf",
      "/Users/poncho/dev/test-files/pdf1_72.pdf"
    ]

    case Pdfr.merge(pdfs, merged_pdf_name) do
      {:ok, _} ->
        Mix.shell().info("PDF merged succesfuly")

      error ->
        Mix.shell().error(inspect(error))
    end
  end
end
