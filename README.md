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

