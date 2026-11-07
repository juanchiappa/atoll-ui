# Arquitectura de Atoll

## Principios
1. **Recursos:** menos de 20 MB de RAM y 0% de CPU en reposo.
2. **Loop orientado a eventos:** el hilo principal duerme bloqueado en el event loop del SO. No existe un render loop. Las animaciones usan timers que viven solo mientras dura la animación.
3. **Rust puro:** sin webview ni frameworks de UI. Cada dependencia nueva se justifica por escrito en el PR que la agrega.
4. **Plugins fuera del Core:** toda integración (Git, IA, N8N, Office, avatar) es un proceso separado en `/plugins`. Sin plugins, el Core no carga nada extra.

## Crates

| Crate | Ruta | Responsabilidad |
|---|---|---|
| `atoll-core` | `core/atoll-core` | Binario `atoll`: loop de eventos, estado y host de plugins |
| `atoll-platform` | `core/atoll-platform` | Todo el código específico de SO (`cfg(target_os)`) |
| `atoll-render` | `core/atoll-render` | Render por software, solo sobre regiones dañadas |
| `atoll-ipc` | `packages/atoll-ipc` | Protocolo IPC local Core ↔ plugins |
| `atoll-plugin-sdk` | `packages/atoll-plugin-sdk` | SDK para autores de plugins |

## Decisiones

- **Render por software** con `tiny-skia` en lugar de GPU (`wgpu`): una isla ocupa pocos píxeles y la GPU suma drivers y memoria. En Windows, el resultado se entrega a una ventana con transparencia por píxel (`WS_EX_LAYERED` + `UpdateLayeredWindow`): Windows conserva la imagen en pantalla y Atoll no hace nada hasta el próximo evento. Para Linux y macOS se evaluará `softbuffer` o el mecanismo nativo equivalente.
- **Backends nativos por SO** detrás de un mismo trait: Win32 (`windows-sys`), AppKit (`objc2`) y Wayland/X11 en Linux.
- **Plugins como procesos separados**, no librerías dinámicas: aislan crashes y consumo de memoria del Core, y pueden escribirse en cualquier lenguaje. Se descubren en `/plugins/<nombre>/plugin.toml` y hablan con el Core por named pipes (Windows) o sockets Unix (Linux/macOS).

- **Animaciones con timer efímero:** la lógica (`atoll-platform::animation`) es pura y no depende del SO. El backend arranca un timer solo cuando el mouse entra o sale de la isla y lo apaga al terminar la animación (≈180 ms). Fuera de eso no hay ningún timer activo: 0% de CPU en reposo.

## Fases
1. Isla dinámica en Windows, midiendo RAM y CPU desde el primer commit.
2. Mismo Core en Linux (Wayland primero).
3. macOS y sistema de plugins.

## Calidad
Sin `unwrap()`, `expect()` ni `panic!` en código de producción (lints del workspace). `cargo fmt`, `cargo clippy -- -D warnings` y `cargo test` deben pasar en Windows, Linux y macOS antes de mergear.
