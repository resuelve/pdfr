defmodule PdfrTest do
  use ExUnit.Case

  @fixtures Path.join(__DIR__, "fixtures")

  defp fixture(name), do: File.read!(Path.join(@fixtures, name))

  # nested_three_pages.pdf tiene dos páginas bajo un nodo /Pages intermedio que es dueño del
  # MediaBox que ellas heredan, y una tercera directo bajo la raíz; flat_two_pages.pdf es plano.
  # Con el ejemplo de merge de lopdf, el nodo intermedio dejaba un /Parent arreglo en la raíz
  # del documento fundido y qpdf lo rechazaba ("root of pages tree has no /Kids array").
  defp merged do
    {:ok, merged} =
      Pdfr.merge_bin([fixture("nested_three_pages.pdf"), fixture("flat_two_pages.pdf")])

    merged
  end

  test "merge_bin deja un solo nodo /Pages, plano, con todas las hojas" do
    merged = merged()

    assert length(Regex.scan(~r{/Type\s*/Pages\b}, merged)) == 1
    assert length(Regex.scan(~r{/Type\s*/Page\b}, merged)) == 5
    assert merged =~ ~r{/Count\s+5\b}

    # cada página apunta a esa raíz con una sola referencia, nunca con un arreglo
    # (los /Parent restantes son los de los bookmarks que el merge agrega por documento)
    [_, root] = Regex.run(~r{/Pages\s+(\d+) 0 R}, merged)
    assert length(Regex.scan(~r{/Parent\s+#{root} 0 R}, merged)) == 5
    refute merged =~ ~r{/Parent\s*\[}
  end

  test "las páginas conservan lo que heredaban del nodo que desaparece" do
    # 2 heredados + 1 propio en el documento anidado, 2 propios en el plano
    assert length(Regex.scan(~r{/MediaBox}, merged())) == 5
  end

  test "merge_bin rechaza lo que no es iodata" do
    assert_raise RuntimeError, fn -> Pdfr.merge_bin([%{}]) end
  end
end
