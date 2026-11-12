# Arquitectura de Atoll

## Principios
1. **Recursos:** menos de 20 MB de RAM y 0% de CPU en reposo.
2. **Loop orientado a eventos:** el hilo principal duerme bloqueado en el event loop del SO. No existe un render loop ni polling. Solo existen cuatro timers permitidos: (a) animación efímera, (b) reloj de un disparo por minuto, (c) debounce de 50 ms al recargar `config.ini` y (d) retardo de hover de un disparo de hasta 2000 ms cuando `hover_delay_ms > 0`. En reposo no hay ningún timer de hover ni de animación activo.
3. **Rust puro:** sin webview ni frameworks de UI. Cada dependencia nueva se justifica por escrito en el PR que la agrega.
4. **Plugins fuera del Core:** toda integración (Git, IA, N8N, Office, avatar, música, dock) es un proceso separado en `plugins/`. Sin plugins, el Core no carga nada extra. Los plugins no abren ventanas propias: mandan un árbol de widgets y el Core lo dibuja.

## Crates

| Crate | Ruta | Responsabilidad |
|---|---|---|
| `atoll-core` | `core/atoll-core` | Binario `atoll`: carga/recarga de `config.ini`, contenido de la isla (`IslandPainter`), estado y host de plugins |
| `atoll-platform` | `core/atoll-platform` | Código específico de SO (`cfg(target_os)`), traits `Backend` y `Painter`, y módulos portables `animation`, `clock` y `dpi` |
| `atoll-render` | `core/atoll-render` | Render por software con `tiny-skia` (con radio independiente por esquina `CornerRadii`) y texto con `fontdue` (fuente embebida y caché de glifos) |
| `atoll-config` | `packages/atoll-config` | Configuración, temas y comportamiento de Atoll: parser y serializador propios sin dependencias externas |
| `atoll-ipc` | `packages/atoll-ipc` | Protocolo IPC local Core ↔ plugins |
| `atoll-plugin-sdk` | `packages/atoll-plugin-sdk` | SDK para autores de plugins |

## Decisiones

- **Render por software** con `tiny-skia` en lugar de GPU (`wgpu`): una isla ocupa pocos píxeles y la GPU suma drivers y memoria. En Windows, el resultado se entrega a una ventana con transparencia por píxel (`WS_EX_LAYERED` + `UpdateLayeredWindow`): Windows conserva la imagen en pantalla y Atoll no hace nada hasta el próximo evento. Se redibuja solo cuando algo cambió. Para Linux y macOS se evaluará `softbuffer` o el mecanismo nativo equivalente.
- **Backends nativos por SO** detrás de un mismo trait (`Backend`): Win32 (`windows-sys`), AppKit (`objc2`) y Wayland en Linux.
- **Plugins como procesos separados**, no librerías dinámicas: aíslan crashes y consumo de memoria del Core, y pueden escribirse en cualquier lenguaje. Viven en `plugins/<nombre>/` con un manifiesto en el formato propio de configuración (sin sumar un parser de TOML), hablan con el Core por named pipes (Windows) o sockets Unix (Linux/macOS), y mandan un árbol de widgets para que el Core lo dibuje.

- **Timers permitidos (solo cuatro):**
  1. **Animación (`ANIMATION_TIMER`):** la lógica (`atoll-platform::animation`) es pura y no depende del SO. El backend arranca un timer de 16 ms solo cuando empieza a expandir o contraer y lo apaga al terminar la animación (`animation_ms`, por defecto 180 ms; con `animation_ms = 0` el cambio es instantáneo y ni siquiera se arma el timer).
  2. **Reloj (`CLOCK_TIMER`):** un timer de un solo disparo por minuto, alineado al cambio de minuto (+ 20 ms de margen) con `SetCoalescableTimer` sin coalescencia (`TIMERV_NO_COALESCING`).
  3. **Debounce de configuración (`RELOAD_TIMER`):** un timer de un solo disparo de 50 ms que se arma (o reinicia) cuando el hilo observador detecta un cambio sobre `config.ini`.
  4. **Retardo de hover (`HOVER_DELAY_TIMER`):** un timer de un solo disparo de hasta 2000 ms que se arma cuando el mouse entra a la isla con `hover_delay_ms > 0` y se cancela (`KillTimer`) si el mouse sale antes de que venza. Con `hover_delay_ms = 0` no se arma ningún timer. Al salir el mouse, la contracción ocurre siempre sin retardo.

- **Comportamiento configurable (`[behavior]`):**
  - La decisión del retardo (`Animator::on_mouse_enter`, `on_mouse_leave`, `on_delay_elapsed`) vive en `atoll-platform::animation` como una máquina de estados pura y portable que devuelve `HoverAction` (`None`, `ArmDelayTimer { delay_ms }`, `CancelDelayTimer`, `StartExpand`, `StartCollapse`) sin llamar al SO, y se prueba con tests unitarios en Linux.
  - Estilo `island` vs. `notch`: en modo `notch`, `island_config_from` fija `top_margin = 0` (la ventana permanece en `y = 0` al expandir y al cambiar el DPI) y `atoll-render` dibuja las esquinas superiores rectas (`0.0`) y las inferiores redondeadas (`CornerRadii::bottom_only`).

- **Quién dibuja qué (`Painter`):** el backend de la plataforma no sabe qué se ve en la isla. Recibe un `Box<dyn Painter>` y le pide un `Frame` cada vez que hay que mostrar algo: al abrir, en cada cuadro de la animación, cuando cambia el minuto, cuando cambia el DPI y cuando cambia la configuración. El Core implementa el `Painter` (`atoll-core::island::IslandPainter`: tema, colores con alfa premultiplicado, radios por esquina según el estilo `island` o `notch`, hora, fecha, layout). Así los plugins podrán pintar contenido en la isla (ítem 15) sin tocar el código de ventana, y la lógica de contenido se prueba en cualquier SO.

- **Configuración, temas y recarga en vivo (excepción a la regla de "sin hilos extra"):**
  - Al arrancar, `atoll-core` busca `%APPDATA%\Atoll\config.ini` con `std::env::var_os("APPDATA")` (sin crates). Si no existe el archivo, crea la carpeta y el archivo con `Config::default().to_text()`. Si `APPDATA` no existe o la creación falla, usa `Config::default()` sin error.
  - **Hilo observador (`atoll-config-watch`):** única excepción documentada a "sin hilos adicionales". En Windows se crea un hilo con pila reducida (64 KB) que abre `%APPDATA%\Atoll` con `CreateFileW(FILE_FLAG_BACKUP_SEMANTICS)` y duerme bloqueado en `ReadDirectoryChangesW` (0% de CPU en reposo, sin polling ni `sleep`). Observa cambios de nombre, escritura, tamaño y creación, y filtra en código seguro las entradas `FILE_NOTIFY_INFORMATION` que correspondan a `config.ini` (contemplando editores que guardan un archivo temporal y lo renombran).
  - Cuando detecta un cambio en `config.ini`, envía `PostMessageW(hwnd, WM_APP + 1, 0, 0)` al hilo de la ventana. Si el hilo no se puede crear, se informa una vez por `stderr` y la isla funciona sin recarga en vivo.
  - **Debounce y aplicación sin parpadeos:** al recibir `WM_APP + 1`, el procedimiento de ventana arma (o reinicia) `RELOAD_TIMER` a 50 ms. Cuando el timer se dispara, se apaga de inmediato (`KillTimer`) y llama a `Painter::reload_config()`:
    - Si el archivo no se puede leer (bloqueado, a medio escribir o borrado), se conserva la configuración actual.
    - Si se lee, se parsea con `atoll_config::parse` (imprimiendo por `stderr` solo las advertencias **nuevas** que no estaban en la carga anterior) y se compara con la `Config` actual.
    - **Solo si la `Config` resultante cambió**, se actualiza el estado, se aplican los nuevos tiempos (`hover_delay_ms`, `animation_ms`) y el margen superior (`0` en `notch`), se llama a `Animator::reconfigure` (que salta de inmediato a los tamaños nuevos conservando el estado de hover), se apagan `ANIMATION_TIMER` y `HOVER_DELAY_TIMER` y se redibuja **una sola vez**.

- **Escalado DPI por monitor:** las medidas de la configuración (`IslandConfig`) y del `Painter` (tamaño de letra, separaciones, radio de esquina) son **lógicas**, a 96 DPI. El backend las multiplica por el factor de escala del monitor (`atoll_platform::dpi`: 120 DPI = 1,25) y le pasa al `Painter` el tamaño físico y la escala. Así la isla mide lo mismo en pantalla y se rasteriza nítida con cualquier escalado, en lugar de que Windows estire un bitmap.
  - En Windows el proceso es Per-Monitor-V2 (`SetProcessDpiAwarenessContext`). Si el SO no lo soporta (anterior a Windows 10 1703) se usa el DPI del sistema (`SetProcessDPIAware`), sin escalado dinámico.
  - El DPI inicial se lee de la propia ventana (`GetDpiForWindow`) antes de mostrarla, creada en el origen de la pantalla principal.
  - `WM_DPICHANGED` y `WM_DISPLAYCHANGE` recalculan tamaños físicos, margen superior (`0` en `notch`) y animador (`Animator::reconfigure`), apagan los timers de animación y retardo si estaban activos y redibujan **una sola vez**.
  - La caché de glifos se vacía cuando cambia la escala (`TextRenderer::clear_cache`): los glifos del tamaño anterior no se vuelven a pedir.
  - La isla vive en el monitor principal. Soportar otros monitores es una decisión futura.

- **Reloj y excepción a la regla de timers:** la isla muestra `HH:MM` en reposo y, con hover, la hora y la fecha (`sáb 7 nov`). La hora local sale de `GetLocalTime` (sin crates de fechas) detrás de `atoll_platform::local_time()`.
  - Se arma con los milisegundos que faltan hasta el próximo cambio de minuto (`LocalTime::millis_until_next_minute`) más 20 ms de margen, para no saltar antes de tiempo.
  - Al dispararse se apaga lo primero (`KillTimer`), se redibuja **solo si el minuto cambió** (se compara con la hora del último cuadro dibujado mediante `LocalTime::same_minute`) y se vuelve a armar. No hay polling ni intervalos fijos: en reposo, un despertar por minuto y nada más.
  - Se usa `SetCoalescableTimer` sin coalescencia (`TIMERV_NO_COALESCING`), porque el `SetTimer` común puede agrupar timers y demorar el cambio de minuto. Si Windows lo rechaza, se cae a `SetTimer`.
  - Se re-sincroniza ante `WM_TIMECHANGE` (el usuario o NTP cambió la hora) y al volver de suspensión (`WM_POWERBROADCAST` con `PBT_APMRESUMEAUTOMATIC` o `PBT_APMRESUMESUSPEND`), porque ahí el timer pudo quedar desfasado.
  - Mientras dura la animación de hover, cada cuadro lee la hora actual, así que el texto siempre está al día. La fecha aparece con un fundido ligado al tamaño de la isla, y la hora se mueve de centrada a bloque superior en el mismo recorrido.

- **Texto con `fontdue` y una fuente embebida:** `atoll-render` rasteriza glifos con `fontdue` y los compone sobre un `Frame` (`TextRenderer::draw`). La fuente (Inter Medium recortada a 87 glifos, 8,8 KB) viaja dentro del binario: no se leen fuentes del sistema y no hay motor de layout. Cada glifo se rasteriza una vez por tamaño y queda en una caché acotada (512 entradas); redibujar el mismo texto no rasteriza nada. El render de texto no toca la ventana ni el backend: quien dibuja le pasa el `Frame`.

## Fases
1. Isla dinámica en Windows (v0.1: reloj, DPI, configuración/temas y comportamiento 12a), midiendo RAM y CPU desde el primer commit.
2. Autoocultar (12b), toolkit `atoll-ui`, sistema de plugins por IPC, idiomas y `atoll-settings`.
3. Mismo Core en Linux (Wayland primero), macOS y plugins oficiales (música, dock).

## Calidad
Sin `unwrap()`, `expect()` ni `panic!` en código de producción (lints del workspace). `cargo fmt`, `cargo clippy -- -D warnings` y `cargo test` deben pasar en Windows, Linux y macOS antes de mergear.
