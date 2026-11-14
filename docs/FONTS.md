# Fuente embebida

Atoll dibuja texto con una sola fuente incluida en el binario (`include_bytes!`). No lee fuentes del sistema.

| Dato | Valor |
|---|---|
| Fuente | Inter, peso Medium |
| Versión | 4.1 (release `v4.1` de rsms/inter, archivo `extras/ttf/Inter-Medium.ttf`) |
| Autores | The Inter Project Authors (https://github.com/rsms/inter) |
| Licencia | SIL Open Font License 1.1 (sin Reserved Font Name) |
| Archivo en el repo | `core/atoll-render/assets/Inter-Medium-atoll.ttf` (18,0 KB, 222 glifos) |
| Texto de la licencia | `core/atoll-render/assets/Inter-OFL.txt` |

La OFL permite embeber, redistribuir y modificar la fuente (acá, recortarla) siempre que cada copia lleve el
aviso de copyright y el texto de la licencia. Eso cubre `THIRD_PARTY_NOTICES.md` y el archivo `Inter-OFL.txt`.
Los binarios que se distribuyan tienen que incluir ambos (se resuelve en el empaquetado de releases).

## Caracteres incluidos

- **ASCII imprimible completo** (`U+0020` a `U+007E`): espacio, dígitos `0-9`, letras `A-Z` y `a-z`, puntuación y símbolos básicos (`%`, `(`, `)`, `+`, `?`, `/`, `|`, etc.).
- **Latin-1 Supplement** (`U+00A0` a `U+00FF`): letras acentuadas, `ñ`, `Ñ`, `ü`, `Ü`, `¿`, `¡`, `·` (`U+00B7`), `°` (`U+00B0`), `×` (`U+00D7`), etc. Todos los puntos de código del rango que tienen glifo en Inter v4.1 están incluidos (`U+00AD` *soft hyphen* es un carácter de formato invisible y no existe en Inter).
- **Símbolos y puntuación tipográfica:** `–` (`U+2013`), `—` (`U+2014`), `‘` (`U+2018`), `’` (`U+2019`), `“` (`U+201C`), `”` (`U+201D`), `•` (`U+2022`), `…` (`U+2026`), `€` (`U+20AC`).

Quedan fuera por ahora otros alfabetos, emojis y símbolos matemáticos. Un carácter que no está se dibuja con el
glifo de "carácter faltante" (`.notdef`) de la fuente. Si hace falta uno nuevo, se regenera el subset y se sube el archivo en el
mismo PR que lo usa; el test `embedded_font_has_every_character_we_need` lo cubre.

## Cómo se generó (reproducible)

Una sola vez, a mano. No es una dependencia del build.

```sh
pip install fonttools
unzip Inter-4.1.zip extras/ttf/Inter-Medium.ttf
pyftsubset extras/ttf/Inter-Medium.ttf \
  --unicodes='U+0020-007E,U+00A0-00FF,U+2013-2014,U+2018-2019,U+201C-201D,U+2022,U+2026,U+20AC' \
  --layout-features='' --no-hinting --notdef-outline --name-IDs='*' --glyph-names \
  --output-file=core/atoll-render/assets/Inter-Medium-atoll.ttf
```

Se sacan las tablas de hinting y de features de layout (kerning, ligaduras): `fontdue` no las usa. Se conserva la
tabla `name`, que lleva el aviso de copyright.
