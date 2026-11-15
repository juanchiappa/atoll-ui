# Changelog

Todos los cambios relevantes de este proyecto se documentan en este archivo.

El formato está basado en [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/),
y este proyecto adhiere a [Semantic Versioning](https://semver.org/lang/es/).

## [Unreleased]

## [0.2.0] - 2026-11-15

### Agregado

- **Mini toolkit declarativo `atoll-ui` (`packages/atoll-ui`):** árbol de widgets de datos planos (`Row`, `Column`, `Box`, `Spacer`, `Divider`, `Label`, `Icon`, `ProgressBar`, `Button`, `IconButton`, `Toggle`, `Slider`), validación con límites (`MAX_TREE_DEPTH = 32`, `MAX_TREE_NODES = 1024`), motor de layout puro y determinista (`compute_layout`), hit-testing (`hit_test`) y estado interactivo con eventos de puntero (`UiState`), paleta de colores derivada del tema (`ThemePalette::from_appearance`), catálogo de 11 íconos vectoriales sin fuentes (`IconId`) y pintado sobre `atoll-render::Frame` (`paint_tree` / `paint_tree_with_text`), sin dependencias externas.
- **Integración de `atoll-ui` en la isla (`atoll-core`, `atoll-platform`):** al terminar de expandirse con el mouse encima, la isla muestra la hora y la fecha a la izquierda y los botones de **Ajustes** (engranaje, abre `config.ini`) y **Cerrar** (`X`, cierra la isla) a la derecha, con soporte de temas, estilos (`island` y `notch`) y escalado DPI. Se agregan los eventos de mouse izquierdo (`WM_LBUTTONDOWN` / `WM_LBUTTONUP` con `SetCapture` / `ReleaseCapture`) sin timers nuevos; el clic derecho sigue cerrando la isla.
- **Primitivas de dibujo en `atoll-render`:** `ClipRect`, `PathCmd`, `fill_rounded_rect`, `stroke_path` y `TextRenderer::draw_clipped`.
- **Fuente embebida ampliada:** `Inter-Medium-atoll.ttf` pasa a incluir ASCII imprimible completo, Latin-1 Supplement y signos tipográficos comunes (`•`, `·`, `…`, `–`, `—`, comillas, `€`, `°`, `×`).
- **Archivos de comunidad:** `CONTRIBUTING.md` (con firma DCO mediante `git commit -s`), `SECURITY.md`, plantillas de issues (reporte de errores y pedido de funciones) y plantilla de Pull Request.

### Cambiado

- **El botón de Ajustes abre `config.ini` con `%SystemRoot%\explorer.exe` por ruta absoluta**, sin búsqueda en `PATH` ni en el directorio actual; si `SystemRoot` o el archivo no existen, informa por `stderr` y no lanza nada.
- **Contenido de la isla durante la animación:** en reposo y durante toda la animación de expansión o contracción se dibuja únicamente la hora centrada; la fecha y los botones aparecen al finalizar la expansión y desaparecen de inmediato cuando comienza la contracción.

### Corregido

- **`config.ini` vacío o con solo espacios y saltos de línea:** al iniciar se rellena con los valores por defecto (los archivos con cualquier otro contenido, incluso solo comentarios, no se tocan).
- **Coordenadas extremas en el rasterizador:** `atoll-render` acota las coordenadas antes de pasarlas a `tiny-skia` para evitar desbordes del rasterizador en punto fijo.

## [0.1.0] - 2026-11-12

Primera versión pública de Atoll (solo Windows x64).

### Agregado

- **Isla dinámica nativa en Windows (Win32):** ventana flotante siempre visible (`WS_EX_LAYERED` + `UpdateLayeredWindow`) con transparencia alfa por píxel y loop 100 % orientado a eventos (`GetMessageW`, 0 % de CPU en reposo). Cierre directo con clic derecho sobre la isla.
- **Render por software y texto nítido (`atoll-render`):** trazado vectorial suavizado con `tiny-skia` y rasterizado de texto con `fontdue` usando un subconjunto embebido de Inter Medium (8,8 KB) y caché acotada de glifos.
- **Reloj y fecha en la isla:** muestra la hora actual (`HH:MM`) en reposo y despliega la fecha abreviada en español (`sáb 7 nov`) al pasar el mouse, sincronizado con el cambio de minuto mediante un único timer de un solo disparo sin coalescencia (`SetCoalescableTimer`), `WM_TIMECHANGE` y reanudación tras suspensión (`WM_POWERBROADCAST`).
- **Escalado DPI por monitor (`Per-Monitor-V2`):** medidas lógicas a 96 DPI que se escalan dinámicamente al factor del monitor (`WM_DPICHANGED`, `WM_DISPLAYCHANGE`) re-rasterizando bordes y glifos sin estirar bitmaps.
- **Archivo de configuración y temas con recarga en vivo (`atoll-config`):** parser y serializador propios sin dependencias externas en `%APPDATA%\Atoll\config.ini`, con creación automática, tolerancia a errores y recarga en vivo mediante `ReadDirectoryChangesW` y debounce de 50 ms:
  - `[island]`: `width`, `height`, `hover_width`, `hover_height` y `top_margin`.
  - `[appearance]`: presets de temas incorporados (`black`, `graphite`, `white`), `background` y `text_color` (`#RRGGBB` / `#RRGGBBAA`), `opacity` (`0..=100` con alfa premultiplicado) y `corner_radius` (`auto` o píxeles lógicos).
- **Comportamiento configurable (`[behavior]`):**
  - `hover_delay_ms` (`0..=2000`): tiempo que el mouse debe permanecer encima antes de expandir la isla, con cancelación inmediata si sale antes. La salida contrae siempre sin retardo.
  - `animation_ms` (`0..=1000`): duración de la animación `ease-out cubic` de expandir y contraer (`0` = instantáneo).
  - `style` (`island` | `notch`): modo isla flotante con las cuatro esquinas redondeadas o modo muesca (`notch`) pegado al borde superior (`y = 0`) con esquinas superiores rectas e inferiores redondeadas.

[Unreleased]: https://github.com/juanchiappa/atoll-ui/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/juanchiappa/atoll-ui/releases/tag/v0.2.0
[0.1.0]: https://github.com/juanchiappa/atoll-ui/releases/tag/v0.1.0
