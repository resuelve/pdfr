defmodule Pdfr.Pdf do
  use Rustler,
    otp_app: :pdfr,
    crate: :pdfr_pdf

  # When loading a NIF module, dummy clauses for all NIF function are required.
  # NIF dummies usually just error out when called when the NIF is not loaded, as that should never normally happen.
  def merge(_documents_to_merge, _merged_pdf_name), do: :erlang.nif_error(:nif_not_loaded)
end
