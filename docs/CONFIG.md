# Formato de configuración de Atoll

La configuración de Atoll usa un formato de texto propio estilo INI, implementado en `packages/atoll-config` sin dependencias externas (`serde`, `toml`, etc.).

## Reglas del formato

- **Comentarios:** toda línea cuyo primer carácter no blanco sea `#` se ignora. También se admiten comentarios al final de una línea precedidos por al menos un espacio (`width = 400 # comentario`), de modo que los colores `#RRGGBB` no se confunden con comentarios.
- **Secciones:** encabezados `[island]`, `[appearance]` y `[behavior]`.
- **Pares clave-valor:** líneas `clave = valor` dentro de una sección.
- **Tolerancia a fallos:** el parser (`atoll_config::parse`) **nunca falla**. Devuelve siempre una `Config` válida junto con una lista de advertencias (`Warning`) con el número de línea (empezando en 1):
  - Un archivo vacío o solo con comentarios produce `Config::default()`.
  - Las secciones o claves desconocidas se ignoran y generan una advertencia (compatibilidad hacia adelante).
  - Los valores inválidos o fuera de rango caen al valor por defecto (o al del preset `theme` en `[appearance]`) y generan una advertencia.

## Secciones y campos

Todas las medidas de tamaño y posición son **píxeles lógicos a 96 DPI** (100 % de escala); el backend las multiplica por el factor de escala del monitor.

### `[island]`

| Clave | Tipo | Rango permitido | Por defecto | Descripción |
|---|---|---|---|---|
| `width` | entero (`u32`) | `120..=1600` | `400` | Ancho de la isla en reposo |
| `height` | entero (`u32`) | `24..=400` | `40` | Alto de la isla en reposo |
| `hover_width` | entero (`u32`) | `120..=1600` | `480` | Ancho de la isla al pasar el mouse |
| `hover_height` | entero (`u32`) | `24..=400` | `64` | Alto de la isla al pasar el mouse |
| `top_margin` | entero (`u32`) | `0..=200` | `8` | Separación respecto del borde superior de la pantalla |

### `[appearance]`

| Clave | Tipo | Valores / Rango | Por defecto | Descripción |
|---|---|---|---|---|
| `theme` | enum | `black`, `graphite`, `white` | `black` | Preset base de colores y opacidad |
| `background` | color | `#RRGGBB` o `#RRGGBBAA` | `#000000` (según `theme`) | Color de fondo de la isla |
| `text_color` | color | `#RRGGBB` o `#RRGGBBAA` | `#FFFFFF` (según `theme`) | Color del texto |
| `opacity` | entero (`u8`) | `0..=100` | `100` (según `theme`) | Opacidad general de la isla (%) |
| `corner_radius` | enum / entero | `auto` o `0..=200` | `auto` | Radio de esquina (`auto` = píldora `min(w, h) / 2`) |

#### Presets de temas (`theme`)

Los valores explícitos válidos en `[appearance]` pisan siempre a los del preset, sin importar el orden en que aparezcan en el archivo:

| Tema | `background` | `text_color` | `opacity` | `corner_radius` |
|---|---|---|---|---|
| `black` | `#000000` | `#FFFFFF` | `100` | `auto` |
| `graphite` | `#1C1C1E` | `#F5F5F7` | `95` | `auto` |
| `white` | `#F5F5F7` | `#111111` | `95` | `auto` |

### `[behavior]`

Reservada para el ítem 12 (retardo de aparición en hover, estilo isla o muesca, autoocultar). Hoy el parser acepta la sección vacía.

## Ejemplo canónico (`Config::default().to_text()`)

```ini
# Configuración de Atoll
# Todas las medidas están en píxeles lógicos a 96 DPI (100 % de escala).

[island]
# Tamaño de la isla en reposo (ancho: 120..1600, alto: 24..400).
width = 400
height = 40
# Tamaño de la isla al pasar el mouse (ancho: 120..1600, alto: 24..400).
hover_width = 480
hover_height = 64
# Separación con el borde superior de la pantalla (0..200).
top_margin = 8

[appearance]
# Tema base: black, graphite o white. Los campos de abajo pisan al tema.
theme = black
# Color de fondo y de texto (#RRGGBB o #RRGGBBAA).
background = #000000
text_color = #FFFFFF
# Opacidad general de la isla (0 a 100).
opacity = 100
# Radio de esquina: "auto" (píldora) o píxeles lógicos (0..200).
corner_radius = auto

[behavior]
# Reservado para opciones de comportamiento.
```
