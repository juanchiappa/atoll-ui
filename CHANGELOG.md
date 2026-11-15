# Changelog

Todos los cambios relevantes de este proyecto se documentan en este archivo.

El formato está basado en [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/),
y este proyecto adhiere a [Semantic Versioning](https://semver.org/lang/es/).

## [Unreleased]

### Agregado

- **Mini toolkit declarativo `atoll-ui` (`packages/atoll-ui`):** árbol de widgets de datos planos (`Row`, `Column`, `Box`, `Spacer`, `Divider`, `Label`, `Icon`, `ProgressBar`, `Button`, `IconButton`, `Toggle`, `Slider`), validación con límites (`MAX_TREE_DEPTH = 32`, `MAX_TREE_NODES = 1024`), motor de layout puro y determinista (`compute_layout`), hit-testing (`hit_test`), máquina de estados interactiva (`UiState`), paleta de colores derivada del tema (`ThemePalette::from_appearance`), catálogo de 11 íconos vectoriales sin fuentes (`IconId`) y pintado sobre `atoll-render::Frame` (`paint_tree` / `paint_tree_with_text`), con 0 dependencias externas.
- **Primitivas de dibujo recortadas y subconjunto de fuente ampliado (`atoll-render`):** `ClipRect`, `PathCmd`, `fill_rounded_rect`, `stroke_path` y `TextRenderer::draw_clipped`, junto con la ampliación de `Inter-Medium-atoll.ttf` a 222 glifos (ASCII imprimible completo, Latin-1 Supplement y signos tipográficos habituales).
- **Vista expandida interactiva en la isla (`atoll-core`, `atoll-platform`):** al completarse la expansión de hover se dibuja un árbol de `atoll-ui` con la hora y la fecha a la izquierda y los botones `IconButton` de **Ajustes** (engranaje, que abre `%APPDATA%\Atoll\config.ini` invocando la ruta absoluta `%SystemRoot%\explorer.exe` sin buscar en `PATH` ni en el directorio actual) y **Cerrar** (`X`, que cierra la isla) a la derecha, con soporte completo de temas, estilos (`island` y `notch`), escalado DPI y captura de mouse (`SetCapture` / `ReleaseCapture`) sin agregar timers nuevos.

### Cambiado

- **Transición de contenido en la isla:** en reposo y durante todo el recorrido de la animación de expansión o contracción se dibuja únicamente la hora centrada; la fecha y los botones aparecen al finalizar la expansión y vuelven de inmediato a la hora centrada cuando comienza la contracción.

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

[0.1.0]: https://github.com/juanchiappa/atoll-ui/releases/tag/v0.1.0

