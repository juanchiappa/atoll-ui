# Formato de configuración de Atoll

La configuración de Atoll usa un formato de texto propio estilo INI, implementado en `packages/atoll-config` sin dependencias externas (`serde`, `toml`, etc.).

## Ubicación y recarga en vivo

- **Ruta en Windows:** `%APPDATA%\Atoll\config.ini`, resuelta con `std::env::var_os("APPDATA")` sin crates adicionales.
- **Creación automática:** si el archivo no existe al iniciar Atoll, se crean la carpeta `%APPDATA%\Atoll` y `config.ini` con `Config::default().to_text()`. Si `APPDATA` no está definido o la creación falla, Atoll arranca con `Config::default()` sin error.
- **Recarga en vivo:** mientras Atoll está abierto, un hilo auxiliar con pila reducida (64 KB) queda bloqueado en `ReadDirectoryChangesW` sobre `%APPDATA%\Atoll` (0% de CPU en reposo, sin polling). Cuando se modifica `config.ini` (o un editor guarda escribiendo un temporal y renombrándolo a `config.ini`), envía `WM_APP + 1` a la ventana, que aplica un debounce de un solo disparo de **50 ms**.
- **Aplicación sin parpadeos:** al cumplirse los 50 ms se relee `config.ini`. Si el archivo está bloqueado, a medio escribir o fue borrado, se conserva la configuración actual. Si se lee y la `Config` resultante **cambió** respecto de la actual, se reconfigura el animador instantáneamente (`Animator::reconfigure`, conservando el estado de hover) y se redibuja una sola vez.
- **Advertencias sin duplicados:** cualquier advertencia del parser se informa por `stderr`, sin abrir ventanas emergentes. En cada recarga en vivo se imprimen únicamente las advertencias **nuevas** que no estaban presentes en la carga o recarga inmediata anterior.

## Reglas del formato

- **Comentarios:** toda línea cuyo primer carácter no blanco sea `#` se ignora. También se admiten comentarios al final de una línea precedidos por al menos un espacio (`width = 400 # comentario`), de modo que los colores `#RRGGBB` no se confunden con comentarios.
- **Secciones:** encabezados `[island]`, `[appearance]` y `[behavior]`.
- **Pares clave-valor:** líneas `clave = valor` dentro de una sección.
- **Tolerancia a fallos:** el parser (`atoll_config::parse`) **nunca falla**. Devuelve siempre una `Config` válida junto con una lista de advertencias (`Warning`) con el número de línea (empezando en 1):
  - Un archivo vacío o solo con comentarios produce `Config::default()`.
  - Las secciones o claves desconocidas se ignoran y generan una advertencia (compatibilidad hacia adelante).
  - Los valores inválidos o fuera de rango caen al valor por defecto (o al del preset `theme` en `[appearance]`) y generan una advertencia.
  - **Relación `hover` vs. reposo en `[island]`:**
    - Si solo se define `width` (o `height`) y su valor es mayor que `hover_width` (`480`) o `hover_height` (`64`) por defecto, **no se descarta** el valor válido: se sube automáticamente `hover_width = width` (o `hover_height = height`) y se informa con una advertencia (`WarningKind::HoverAdjusted`).
    - Solo se vuelve a ambos valores por defecto cuando ambos están definidos (o solo se definió `hover_*`) y `hover < reposo` (`WarningKind::HoverSmallerThanRest`).

## Secciones y campos

Todas las medidas de tamaño y posición son **píxeles lógicos a 96 DPI** (100 % de escala); el backend las multiplica por el factor de escala del monitor.

### `[island]`

| Clave | Tipo | Rango permitido | Por defecto | Descripción |
|---|---|---|---|---|
| `width` | entero (`u32`) | `120..=1600` | `400` | Ancho de la isla en reposo |
| `height` | entero (`u32`) | `24..=400` | `40` | Alto de la isla en reposo |
| `hover_width` | entero (`u32`) | `120..=1600` (`>= width`) | `480` | Ancho de la isla al pasar el mouse |
| `hover_height` | entero (`u32`) | `24..=400` (`>= height`) | `64` | Alto de la isla al pasar el mouse |
| `top_margin` | entero (`u32`) | `0..=200` | `8` | Separación respecto del borde superior de la pantalla (se ignora en `style = notch`) |

### `[appearance]`

| Clave | Tipo | Valores / Rango | Por defecto | Descripción |
|---|---|---|---|---|
| `theme` | enum | `black`, `graphite`, `white` | `black` | Preset base de colores y opacidad |
| `background` | color | `#RRGGBB` o `#RRGGBBAA` | `#000000` (según `theme`) | Color de fondo de la isla |
| `text_color` | color | `#RRGGBB` o `#RRGGBBAA` | `#FFFFFF` (según `theme`) | Color del texto |
| `opacity` | entero (`u8`) | `0..=100` | `100` (según `theme`) | Opacidad general de la isla (%) |
| `corner_radius` | enum / entero | `auto` o `0..=200` | `auto` | Radio de esquina (`auto` = píldora `min(w, h) / 2`; en `notch` aplica a las esquinas inferiores) |

#### Presets de temas (`theme`)

Los valores explícitos válidos en `[appearance]` pisan siempre a los del preset, sin importar el orden en que aparezcan en el archivo:

| Tema | `background` | `text_color` | `opacity` | `corner_radius` |
|---|---|---|---|---|
| `black` | `#000000` | `#FFFFFF` | `100` | `auto` |
| `graphite` | `#1C1C1E` | `#F5F5F7` | `95` | `auto` |
| `white` | `#F5F5F7` | `#111111` | `95` | `auto` |

### `[behavior]`

| Clave | Tipo | Valores / Rango | Por defecto | Descripción |
|---|---|---|---|---|
| `hover_delay_ms` | entero (`u32`) | `0..=2000` | `0` | Tiempo en ms que el mouse debe permanecer encima antes de expandir. Si sale antes, no pasa nada. La salida siempre contrae sin retardo |
| `animation_ms` | entero (`u32`) | `0..=1000` | `180` | Duración en ms de la animación de expandir y contraer (`0` = cambio instantáneo) |
| `style` | enum | `island`, `notch` | `island` | `island`: isla flotante con `top_margin` y las 4 esquinas redondeadas. `notch`: muesca pegada al borde superior (`y = 0`), esquinas superiores rectas y esquinas inferiores redondeadas |

## Ejemplo canónico (`Config::default().to_text()`)

```ini
# Configuración de Atoll
# Todas las medidas están en píxeles lógicos a 96 DPI (100 % de escala).

[island]
# Tamaño de la isla en reposo (ancho: 120..1600, alto: 24..400).
width = 400
height = 40
# Tamaño de la isla al pasar el mouse (ancho: 120..1600 >= width, alto: 24..400 >= height).
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
# Tiempo que el mouse debe permanecer encima antes de expandir, en ms (0..2000).
hover_delay_ms = 0
# Duración de la animación de expandir y contraer, en ms (0..1000).
animation_ms = 180
# Estilo visual: "island" (flotante) o "notch" (muesca pegada al borde superior).
style = island
```
