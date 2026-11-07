# Fuente embebida

Atoll dibuja texto con una sola fuente incluida en el binario (`include_bytes!`). No lee fuentes del sistema.

| Dato | Valor |
|---|---|
| Fuente | Inter, peso Medium |
| Versión | 4.1 (release `v4.1` de rsms/inter, archivo `extras/ttf/Inter-Medium.ttf`) |
| Autores | The Inter Project Authors (https://github.com/rsms/inter) |
| Licencia | SIL Open Font License 1.1 (sin Reserved Font Name) |
| Archivo en el repo | `core/atoll-render/assets/Inter-Medium-atoll.ttf` (8,8 KB, 87 glifos) |
| Texto de la licencia | `core/atoll-render/assets/Inter-OFL.txt` |

La OFL permite embeber, redistribuir y modificar la fuente (acá, recortarla) siempre que cada copia lleve el
aviso de copyright y el texto de la licencia. Eso cubre `THIRD_PARTY_NOTICES.md` y el archivo `Inter-OFL.txt`.
Los binarios que se distribuyan tienen que incluir ambos (se resuelve en el empaquetado de releases).

## Caracteres incluidos

Espacio, dígitos `0-9`, `: . , / -`, letras `a-z` y `A-Z`, y `á é í ó ú ñ Á É Í Ó Ú Ñ`.
Alcanza para la hora (`HH:MM`) y la fecha en español (`sáb 7 nov`). Un carácter que no está se dibuja con el
glifo de "carácter faltante" de la fuente. Si hace falta uno nuevo, se regenera el subset y se sube el archivo en el
mismo PR que lo usa; el test `embedded_font_has_every_character_we_need` lo cubre.

## Cómo se generó (reproducible)

Una sola vez, a mano. No es una dependencia del build.

```sh
pip install fonttools
unzip Inter-4.1.zip extras/ttf/Inter-Medium.ttf
pyftsubset extras/ttf/Inter-Medium.ttf \
  --unicodes='U+0020,U+002C-002F,U+0030-003A,U+0041-005A,U+0061-007A,U+00C1,U+00C9,U+00CD,U+00D1,U+00D3,U+00DA,U+00E1,U+00E9,U+00ED,U+00F1,U+00F3,U+00FA' \
  --layout-features='' --no-hinting --notdef-outline --name-IDs='*' --glyph-names \
  --output-file=core/atoll-render/assets/Inter-Medium-atoll.ttf
```

Se sacan las tablas de hinting y de features de layout (kerning, ligaduras): `fontdue` no las usa. Se conserva la
tabla `name`, que lleva el aviso de copyright.
