# Roadmap de Atoll

Fuente de verdad del backlog: los demás chats la leen desde el repo. Cada ítem es un PR separado
(rama -> PR -> 3 checks -> Squash and merge). Los cambios de arquitectura o dependencias actualizan
`docs/ARCHITECTURE.md` y `docs/DEPENDENCIES.md` en el mismo PR.

Las ideas todavía sin decidir viven en `docs/IDEAS.md`, no acá.

## Reglas innegociables

1. **Recursos:** < 20 MB de RAM y 0 % de CPU en reposo. Loop 100 % orientado a eventos, sin render loop.
   Solo cuatro timers permitidos: (a) animación efímera, (b) reloj de un disparo por minuto (ítem 9),
   (c) debounce de 50 ms al recargar `config.ini` (ítem 11) y (d) retardo de hover de un disparo de hasta
   2000 ms al entrar el mouse cuando `hover_delay_ms > 0` (ítem 12a).
2. **Core en Rust puro.** Sin webview ni frameworks de UI. Cada crate nuevo se justifica en
   `docs/DEPENDENCIES.md`, en el mismo PR que lo agrega.
3. **Monorepo:** `core/`, `packages/`, `plugins/`, `docs/`, `.github/`.
4. **Integraciones** (Git, IA, N8N, Office, avatar, música, dock) solo como plugins en `plugins/`:
   procesos separados con IPC local. Sin plugins, el Core no carga nada extra. Los plugins no abren
   ventanas propias: mandan un árbol de widgets y el Core lo dibuja.
5. **Open source (Apache-2.0):** sin `unwrap`/`expect`/`panic` en producción, clippy y fmt limpios,
   docs en los `pub` items, sin dependencias GPL/AGPL.

## Estado

Referencia: Hecho / En revisión / Pendiente.

| # | Ítem | Estado |
|---|---|---|
| 1 | Estructura inicial del monorepo, README y `CLAUDE.md` | Hecho |
| 2 | Licencia Apache-2.0, política de marca (`TRADEMARK.md`) y CI en ubuntu, windows y macos | Hecho |
| 3 | Workspace de Cargo con los 5 crates base y docs de arquitectura | Hecho |
| 4 | Ventana de la isla en Windows con loop orientado a eventos (#1) | Hecho |
| 5 | Render con `tiny-skia` y transparencia por píxel (#2) | Hecho |
| 6 | Protección de `main`: PR obligatorio y los 3 checks requeridos | Hecho |
| 7 | Animación de hover: la isla pasa de 400x40 a 480x64 y vuelve, con timer efímero (#3) | Hecho |
| 8 | Primitiva de texto en `atoll-render` (#5) | Hecho |
| 9 | Reloj en la isla (#6) | Hecho |
| 10 | Escalado DPI por monitor (#7) | Hecho |
| 11 | Archivo de configuración y temas (colores, tamaños, radio, opacidad, margen) (#11, #12) | Hecho |
| 12a | Comportamiento configurable (`hover_delay_ms`, `animation_ms`, `style = island \| notch`) | Hecho |
| 13 | Release v0.1: workflow de release con ejecutable de Windows y README con la medición | Pendiente |
| 12b | Autoocultar (v0.2) | Pendiente |
| 14 | `atoll-ui`, mini toolkit de widgets | Pendiente |
| 15 | Protocolo IPC y host de plugins (con árbol de widgets) | Pendiente |
| 16 | Idiomas es/en | Pendiente |
| 17 | `atoll-settings`, menú de configuración (ejecutable aparte) | Pendiente |
| 18 | Plugin de música (Windows) | Pendiente |
| 19 | Backend de Linux (Wayland primero) | Pendiente |
| 20 | Backend de macOS | Pendiente |
| 21 | Dock propio (plugin, solo Windows) | Pendiente |

**v0.1 pública = ítems 8 a 13 (con 12a, sin 12b), solo Windows.** El ítem 12b (autoocultar) pasa a la v0.2. Linux y macOS no se prometen hasta que existan.

## Mediciones de referencia

- Windows, build debug, isla abierta en reposo: 7,0 MB de RAM y 0,00 de CPU (hasta el ítem 5).
- Cada PR que toque render, ventana o timers tiene que volver a medir y reportar RAM y CPU en reposo.

## Ítems recientes completados

### 8. Primitiva de texto en `atoll-render` (hecho)

- Solo `atoll-render`: no toca la ventana ni el backend.
- `TextRenderer::draw` dibuja un texto sobre un `Frame`; `measure` y `line_metrics` ayudan a centrarlo.
- Dependencia: `fontdue` (`0.9`, `default-features = false`). Fuente Inter Medium recortada, embebida con
  `include_bytes!` (`docs/FONTS.md`, `THIRD_PARTY_NOTICES.md`).
- Caché de glifos acotada (512 entradas): lo que no cambia no se rasteriza de nuevo.
- **Aceptación:** tests de píxeles (cobertura, color, recorte, caché), fmt, clippy y tests en verde en los 3 SO,
  RAM en reposo en Windows <= 8 MB.

### 9. Reloj en la isla (hecho)

- La hora local sale de `GetLocalTime` (Windows). Sin crates de fechas.
- El backend recibe un `Painter` (trait en `atoll-platform`) y el Core pinta el contenido (`IslandPainter`).
- En reposo la isla muestra `HH:MM`; con hover muestra la hora y la fecha (`sáb 7 nov`).
- **Excepción deliberada a la regla de timers:** UN timer de un solo disparo por minuto, alineado al cambio de
  minuto. Se calcula cuántos milisegundos faltan (+ 20 ms de margen) y se re-arma en cada disparo. Sin polling.
  Solo se redibuja si el minuto cambió. Documentado en `docs/ARCHITECTURE.md`.
- Se re-arma el timer también ante `WM_TIMECHANGE` y al volver de suspensión (`WM_POWERBROADCAST`), para no
  quedar desfasado.
- La animación de hover sigue funcionando con el texto adentro.
- **Aceptación:** CPU en reposo 0,00 medido; la hora cambia exactamente al minuto; RAM <= 8 MB.

### 10. Escalado DPI por monitor (hecho)

- Las medidas de `IslandConfig` pasan a ser **lógicas** (96 DPI). El backend las multiplica por el factor de
  escala del monitor (`atoll_platform::dpi`), y el `Painter` recibe el tamaño físico y la escala.
- Per-Monitor-V2 (`SetProcessDpiAwarenessContext`, con respaldo a `SetProcessDPIAware` en Windows anteriores a
  1703): `WM_DPICHANGED` y `WM_DISPLAYCHANGE` recalculan tamaños físicos, margen y animador
  (`Animator::resize`, sin animar, conservando el hover) y redibujan una sola vez. Se invalida la caché de
  glifos (`TextRenderer::clear_cache`) cuando cambia la escala.
- **Aceptación:** misma isla, mismo tamaño físico y nítida con 100, 125, 150 y 200 % de escala y en dos monitores
  con DPI distinto; cambiar el escalado en Configuración reacomoda la isla sin reiniciar; sin render loop ni
  timers nuevos; CPU en reposo 0,00.

### 11. Archivo de configuración y temas (hecho)

- Configuración con formato propio mínimo (`clave = valor` por secciones en `packages/atoll-config`, parser y
  serializador propios con tests), sin `serde` ni `toml`. Ubicación en Windows: `%APPDATA%\Atoll\config.ini`.
- Valores inválidos no rompen nada: se usa el valor por defecto y se informa por `stderr` (sin repetir advertencias
  idénticas al recargar). Si solo se define `width` o `height` mayor al `hover_*` por defecto, se ajusta el hover
  automáticamente con advertencia.
- Permite configurar tema (`black`, `graphite`, `white`), colores (`background`, `text_color`), tamaños,
  radio de borde (`auto` o píxeles), opacidad (`0..=100`, con alfa premultiplicado) y margen superior.
- Recarga en vivo al detectar cambios sobre `%APPDATA%\Atoll\config.ini` con `ReadDirectoryChangesW` (hilo
  auxiliar bloqueado en el SO, 0% CPU en reposo) y debounce de un disparo de 50 ms.

### 12a. Comportamiento configurable (hecho)

- Sección `[behavior]` en `config.ini` con recarga en vivo:
  - `hover_delay_ms` (`0..=2000`, por defecto `0`): retardo antes de expandir al entrar el mouse, implementado
    con una máquina de estados portable (`HoverAction`) y un timer de un solo disparo (`HOVER_DELAY_TIMER`) que
    se cancela (`KillTimer`) si el mouse sale antes de que venza. La salida contrae siempre sin retardo.
  - `animation_ms` (`0..=1000`, por defecto `180`): duración de expandir y contraer (`0` = instantáneo).
  - `style` (`island | notch`, por defecto `island`): en `notch` la ventana se ubica en `y = 0` ignorando
    `top_margin`, con esquinas superiores rectas y esquinas inferiores redondeadas (`CornerRadii`).

## Ítems pendientes

### 13. Release v0.1

- Workflow de release en GitHub Actions que publica el ejecutable de Windows (con `THIRD_PARTY_NOTICES.md` y
  licencias).
- `README.md` actualizado con el alcance real (solo Windows en v0.1) y las mediciones reproducibles de RAM y CPU.

### 12b. Autoocultar (v0.2)

- Opción de autoocultar la isla y mostrarla al acercar el puntero al borde superior (movido de la v0.1 a la v0.2).

### 14. `atoll-ui`, mini toolkit de widgets

- Crate `packages/atoll-ui` sobre `atoll-render`: texto, íconos, filas y columnas, botones, toggles, sliders y
  detección de clics. Base del contenido de la isla, de los widgets de plugins y de `atoll-settings`.

### 15. Protocolo IPC y host de plugins

- Protocolo en `atoll-ipc`, SDK en `atoll-plugin-sdk`, host en `atoll-core`. Named pipes en Windows y sockets
  Unix en Linux/macOS. Los plugins mandan un árbol de widgets y el Core lo dibuja.
- Manifiesto de plugin con el mismo formato propio de configuración (sin agregar parser de TOML).
- Un ADR en `docs/` fija transporte, framing, versionado y permisos antes de escribir código.
- Antes del primer plugin oficial hay que agregar `exclude = ["plugins"]` al `Cargo.toml` raíz y actualizar el
  `.gitignore` (quitar `/plugins/*` y `!plugins/.gitkeep`, e ignorar `/plugins/**/target/`).
- **Aceptación:** un plugin de ejemplo pinta contenido en la isla; con 0 plugins la RAM no cambia.

### 16. Idiomas es/en

- Tablas embebidas para español e inglés, sin crates de i18n.

### 17. `atoll-settings`, menú de configuración

- Ejecutable separado (`core/atoll-settings`) construido con `atoll-ui`. Escribe el archivo de configuración y se
  cierra, de modo que cuando está cerrado no consume RAM.

### 18. Plugin de música (Windows)

- Primer plugin oficial en `plugins/` (proyecto Cargo independiente). Usa las APIs de medios de Windows fuera del
  Core para no ensuciar su binario ni su consumo base.

### 19. Backend de Linux (Wayland primero)

- Isla con alfa por píxel en Wayland (layer-shell). ADR de la dependencia antes de código. X11 queda fuera.
- **Aceptación:** isla visible en compositores con layer-shell (Sway, Hyprland, KDE), 0 % de CPU en reposo;
  límites en GNOME/Mutter documentados.

### 20. Backend de macOS

- Panel sin bordes, flotante, sin ícono en el Dock. Binding mínimo a AppKit.
- **Aceptación:** isla visible sobre otras apps, 0 % de CPU en reposo.

### 21. Dock propio (plugin, solo Windows)

- Plugin opcional para Windows, sin reemplazar la bandeja del sistema (área de notificaciones).
