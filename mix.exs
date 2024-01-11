defmodule Pdfr.MixProject do
  use Mix.Project

  def project do
    [
      app: :pdfr,
      version: "0.3.0",
      elixir: "~> 1.15",
      start_permanent: Mix.env() == :prod,
      aliases: aliases(),
      deps: deps()
    ]
  end

  # Run "mix help compile.app" to learn about applications.
  def application do
    [
      extra_applications: [:logger]
    ]
  end

  # Run "mix help deps" to learn about dependencies.
  defp deps do
    [
      {:rustler, "~> 0.30.0"}
    ]
  end

  defp aliases do
    [
      ci: [
        "hex.audit",
        "deps.unlock --check-unused",
        "format --check-formatted",
        "test"
      ]
    ]
  end
end
