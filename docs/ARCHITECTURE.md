# Arquitectura de Atoll

## Principios
1. **Recursos:** menos de 20 MB de RAM y 0% de CPU en reposo.
2. **Loop orientado a eventos:** el hilo principal duerme bloqueado en el event loop del SO. No existe un render loop. Las animaciones usan timers que viven solo mientras dura la animación. Única excepción, deliberada: el timer del reloj (ver "Reloj y excepción a la regla de timers").
3. **Rust puro:** sin webview ni frameworks de UI. Cada dependencia nueva se justifica por escrito en el PR que la agrega.
4. **Plugins fuera del Core:** toda integración (Git, IA, N8N, Office, avatar) es un proceso separado en `/plugins`. Sin plugins, el Core no carga nada extra.

## Crates

| Crate | Ruta | Responsabilidad |
|---|---|---|
| `atoll-core` | `core/atoll-core` | Binario `atoll`: contenido de la isla (`Painter`), estado y host de plugins |
| `atoll-platform` | `core/atoll-platform` | Todo el código específico de SO (`cfg(target_os)`) |
| `atoll-render` | `core/atoll-render` | Render por software, solo sobre regiones dañadas |
| `atoll-ipc` | `packages/atoll-ipc` | Protocolo IPC local Core ↔ plugins |
| `atoll-plugin-sdk` | `packages/atoll-plugin-sdk` | SDK para autores de plugins |

## Decisiones

- **Render por software** con `tiny-skia` en lugar de GPU (`wgpu`): una isla ocupa pocos píxeles y la GPU suma drivers y memoria. En Windows, el resultado se entrega a una ventana con transparencia por píxel (`WS_EX_LAYERED` + `UpdateLayeredWindow`): Windows conserva la imagen en pantalla y Atoll no hace nada hasta el próximo evento. Para Linux y macOS se evaluará `softbuffer` o el mecanismo nativo equivalente.
- **Backends nativos por SO** detrás de un mismo trait: Win32 (`windows-sys`), AppKit (`objc2`) y Wayland/X11 en Linux.
- **Plugins como procesos separados**, no librerías dinámicas: aislan crashes y consumo de memoria del Core, y pueden escribirse en cualquier lenguaje. Se descubren en `/plugins/<nombre>/plugin.toml` y hablan con el Core por named pipes (Windows) o sockets Unix (Linux/macOS).

- **Animaciones con timer efímero:** la lógica (`atoll-platform::animation`) es pura y no depende del SO. El backend arranca un timer solo cuando el mouse entra o sale de la isla y lo apaga al terminar la animación (≈180 ms). Fuera de eso no hay ningún timer activo: 0% de CPU en reposo.

- **Quién dibuja qué (`Painter`):** el backend de la plataforma no sabe qué se ve en la isla. Recibe un `Box<dyn Painter>` y le pide un `Frame` cada vez que hay que mostrar algo: al abrir, en cada cuadro de la animación y cuando cambia el minuto. El Core implementa el `Painter` (`atoll-core::island::IslandPainter`: hora, fecha, layout). Así los plugins podrán pintar contenido en la isla (ítem 11) sin tocar el código de ventana, y la lógica de contenido se prueba en cualquier SO.

- **Reloj y excepción a la regla de timers:** la isla muestra `HH:MM` en reposo y, con hover, la hora y la fecha (`sáb 7 nov`). La hora local sale de `GetLocalTime` (sin crates de fechas) detrás de `atoll_platform::local_time()`. La regla "los timers solo existen durante una animación" tiene **una sola excepción**: UN timer de un solo disparo por minuto.
  - Se arma con los milisegundos que faltan hasta el próximo cambio de minuto (`LocalTime::millis_until_next_minute`) más 20 ms de margen, para no saltar antes de tiempo.
  - Al dispararse se apaga lo primero, se redibuja **solo si el minuto cambió** (se compara con la hora del último cuadro dibujado) y se vuelve a armar. No hay polling ni intervalos fijos: en reposo, un despertar por minuto y nada más.
  - Se usa `SetCoalescableTimer` sin coalescencia, porque el `SetTimer` común puede agrupar timers y demorar el cambio de minuto. Si Windows lo rechaza, se cae a `SetTimer`.
  - Se re-sincroniza ante `WM_TIMECHANGE` (el usuario o NTP cambió la hora) y al volver de suspensión (`WM_POWERBROADCAST`), porque ahí el timer pudo quedar desfasado.
  - Mientras dura la animación de hover, cada cuadro lee la hora actual, así que el texto siempre está al día. La fecha aparece con un fundido ligado al tamaño de la isla, y la hora se mueve de centrada a bloque superior en el mismo recorrido.

- **Texto con `fontdue` y una fuente embebida:** `atoll-render` rasteriza glifos con `fontdue` y los compone sobre un `Frame` (`TextRenderer::draw`). La fuente (Inter Medium recortada a 87 glifos, 8,8 KB) viaja dentro del binario: no se leen fuentes del sistema y no hay motor de layout. Cada glifo se rasteriza una vez por tamaño y queda en una caché acotada (512 entradas); redibujar el mismo texto no rasteriza nada. El render de texto no toca la ventana ni el backend: quien dibuja le pasa el `Frame`.

## Fases
1. Isla dinámica en Windows, midiendo RAM y CPU desde el primer commit.
2. Mismo Core en Linux (Wayland primero).
3. macOS y sistema de plugins.

## Calidad
Sin `unwrap()`, `expect()` ni `panic!` en código de producción (lints del workspace). `cargo fmt`, `cargo clippy -- -D warnings` y `cargo test` deben pasar en Windows, Linux y macOS antes de mergear.
