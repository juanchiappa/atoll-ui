# Atoll — documento de traspaso

Este documento reúne todo el contexto del proyecto, las decisiones tomadas y su porqué, para continuar el desarrollo con otro agente o herramienta sin perder el hilo. Las reglas operativas cortas están en `AGENTS.md`. **El repo es la fuente de verdad**: ante cualquier diferencia entre este documento y el código, manda el código; y hay que corregir el documento.

## 1. Qué es Atoll

UI liviana y nativa para Windows, Linux y macOS. Nació como isla dinámica (al estilo de Nusa, Fenn y Bloom) y apunta a que el usuario pueda modificar el aspecto y el comportamiento de su sistema como si fuera uno nuevo. Se promociona en redes sociales (Instagram y GitHub).

Diferencial: consumo mínimo. Medición en Windows con la isla en reposo: ~7 MB de RAM y 0,00 de CPU. Todo lo que se agregue se mide contra eso.

Dueño del proyecto: Juan (`juanchiappa`), Buenos Aires. Idioma de trabajo: español rioplatense.

## 2. Reglas innegociables

1. Menos de 20 MB de RAM y 0% de CPU en reposo. Loop orientado a eventos, sin render loop.
2. Core en Rust puro, sin webview ni frameworks de UI. Cada crate nuevo se justifica por escrito.
3. Monorepo: `core/`, `packages/`, `plugins/`, `docs/`, `.github/`.
4. Integraciones solo como plugins (procesos separados, IPC local).
5. Código listo para público: sin `unwrap`/`expect`/`panic`, clippy y fmt limpios, docs en `pub`.

## 3. Estado del proyecto

| Ítem | Descripción | Estado |
|---|---|---|
| 1-4 | Repo, licencia, marca, CI, workspace de Cargo con 5 crates | Hecho y verificado |
| 5 | Ventana de isla en Windows con loop de eventos | Hecho y verificado |
| 6 | Render con tiny-skia y ventana con alfa por píxel | Hecho y verificado |
| 7 | Animación de hover con timer efímero | Hecho y verificado |
| 8 | Texto en la isla | Reportado como terminado por el dueño |
| 9 | Reloj en la isla | Reportado como terminado por el dueño |
| 10 | Escalado DPI por monitor | Reportado como terminado por el dueño |

Los ítems 8 a 10 se hicieron en otro chat. **Primera tarea del agente nuevo: auditar el repo** y dejar `docs/ROADMAP.md`, `docs/ARCHITECTURE.md` y `docs/DEPENDENCIES.md` coherentes con el código real (qué dependencia de texto se eligió, cómo quedó el timer del reloj, cómo se maneja el DPI).

### Crates
- `atoll-core`: binario `atoll`. Arma la configuración, renderiza la isla y la entrega al backend.
- `atoll-platform`: trait `Backend`, `native_backend()`, `IslandConfig`, `PlatformError`, módulo portable `animation` (`Animator`, `ease_out_cubic`) y módulo `windows` (solo `cfg(windows)`).
- `atoll-render`: `Size`, `Frame` (RGBA premultiplicado, con `bgra_premultiplied()`), `render_island()`, errores de render, texto.
- `atoll-ipc`: hoy solo `PROTOCOL_VERSION`. Se desarrolla en el ítem 15.
- `atoll-plugin-sdk`: hoy solo reexporta la versión del protocolo.

### Valores actuales de la isla
Reposo 400x40, hover 480x64, margen superior 8 px, animación de 180 ms con `ease-out cubic`, centrada horizontalmente, color negro.

## 4. Decisiones de arquitectura y su porqué

1. **Sin webview ni framework de UI.** Es lo único que permite el presupuesto de RAM.
2. **Render por software con `tiny-skia`, sin GPU.** La isla ocupa pocos píxeles; `wgpu` suma drivers y memoria. Se configuró sin SIMD ni PNG para achicar el binario.
3. **Ventana `WS_EX_LAYERED` + `UpdateLayeredWindow`.** Se entrega la imagen una vez y Windows la mantiene en pantalla; Atoll no hace nada hasta el próximo evento. Reemplazó a `SetWindowRgn`, que dejaba bordes serrados. Los clics sobre píxeles transparentes pasan a la ventana de abajo.
4. **`GetMessageW` bloqueante.** El hilo duerme hasta que llega un mensaje: 0% de CPU en reposo.
5. **Timers solo efímeros.** El timer de animación existe únicamente mientras la isla se mueve. Excepción documentada: un timer de un disparo por minuto para el reloj, alineado al cambio de minuto.
6. **Estado de la ventana en `thread_local!` con `Cell`**, no con `RefCell`: Windows puede reentrar al procedimiento de ventana de forma síncrona (por ejemplo desde `UpdateLayeredWindow`) y un `RefCell` entraría en pánico.
7. **`windows-sys` y no el crate `windows`.** Bindings crudos, sin capas. En Linux y macOS no suma nada (`[target.'cfg(windows)'.dependencies]`).
8. **`WM_MOUSELEAVE` definido a mano (`0x02A3`).** `windows-sys` lo ubica en el módulo `Controls`; activar esa feature por una constante no vale.
9. **Plugins como procesos separados.** Aíslan caídas y consumo del Core, y se pueden escribir en cualquier lenguaje. IPC: named pipes en Windows, sockets Unix en Linux y macOS.
10. **Los plugins mandan un árbol de widgets y el Core lo dibuja.** Cada plugin es un proceso diminuto, sin su propio rasterizador ni fuentes. El protocolo del ítem 15 se diseña con esto.
11. **Toolkit propio `atoll-ui`** (texto, íconos, filas y columnas, botones, toggles, sliders, detección de clics) sobre el render existente. Es la base de los widgets de plugins, del contenido de la isla y del menú de configuración.
12. **Menú de configuración como ejecutable aparte (`atoll-settings`).** Escribe el archivo de configuración y se cierra. Cuando está cerrado no ocupa RAM. El Core recarga al detectar el cambio del archivo con la notificación del SO (`ReadDirectoryChangesW` en Windows), sin polling.
13. **Configuración con formato propio mínimo** (clave = valor por secciones, parser propio con tests), sin `serde` ni `toml`. Ubicación en Windows: `%APPDATA%\Atoll\`. Valores inválidos no rompen nada: se usa el valor por defecto y se informa.
14. **Idiomas: es/en como tablas embebidas**, sin crates de i18n. Escrituras complejas (hindi, árabe, CJK) quedan para más adelante: requieren motor de shaping y fuentes grandes.
15. **Un solo monorepo.** Los plugins oficiales viven en `plugins/<nombre>/` como proyectos Cargo independientes (su propio `Cargo.toml` y `Cargo.lock`), excluidos del workspace raíz con `exclude = ["plugins"]`, para que sus dependencias pesadas no ensucien el `Cargo.lock` del Core. Cuando llegue el primer plugin oficial hay que cambiar el `.gitignore`: quitar `/plugins/*` y `!plugins/.gitkeep`, e ignorar `/plugins/**/target/`. Archivos pesados (videos, renders grandes) no van al repo.
16. **Dock propio: plugin opcional, solo Windows, sin reemplazar la bandeja del sistema.** Reemplazar el área de notificaciones es lo más frágil de toda la idea.
17. **Pendiente de decidir en el ítem 15:** el manifiesto de los plugins se planeó como `plugin.toml`, pero la decisión 13 evita sumar un parser de TOML. Usar el mismo formato propio de la configuración.

## 5. Licencia y marca

- Código: **Apache-2.0** (`LICENSE`). Se evaluó y descartó PolyForm Noncommercial: no es open source en sentido estricto y el dueño prefirió open source real.
- El nombre "Atoll", el logo y la identidad visual están protegidos por `TRADEMARK.md` (la licencia del código no los cubre). Los forks deben cambiar nombre y logo.
- Se recomendó adoptar DCO (`Signed-off-by` en cada commit) antes de aceptar contribuciones externas, para poder relicenciar en el futuro. **Todavía no está implementado.**

## 6. Configuración de GitHub

- Ruleset `proteger-main`, activo: Pull Request obligatorio (0 aprobaciones requeridas, es un solo desarrollador), tres checks requeridos (`ci (ubuntu-latest)`, `ci (windows-latest)`, `ci (macos-latest)`), sin force push, sin borrado de la rama. No se puede pushear directo a `main`.
- Merge: Squash and merge, borrando la rama al terminar.
- CI (`.github/workflows/ci.yml`): matriz de tres sistemas con `fail-fast: false`; corre `cargo fmt --check`, `cargo clippy -D warnings` y `cargo test`.
- Pendiente: workflow de release (ítem 13), DCO, plantillas de issues y PR, `SECURITY.md`.
- Panel About: descripción en inglés sin prefijo y topics (`rust`, `dynamic-island`, `desktop`, `ui`, `customization`, `cross-platform`, `lightweight`, `windows`, `linux`, `macos`, `open-source`). Verificar que `source-available` ya no figure.

## 7. Backlog y alcance

| # | Ítem |
|---|---|
| 11 | Archivo de configuración y temas (colores, tamaños, radio, opacidad, margen) |
| 12 | Comportamiento configurable (retardo de aparición, estilo isla o muesca, autoocultar) |
| 13 | Release v0.1: workflow de release que publica el ejecutable de Windows, README con la medición |
| 14 | `atoll-ui`, mini toolkit de widgets |
| 15 | Protocolo IPC y host de plugins (con árbol de widgets) |
| 16 | Idiomas es/en |
| 17 | `atoll-settings`, menú de configuración |
| 18 | Plugin de música (Windows) |
| 19 | Backend de Linux (Wayland primero) |
| 20 | Backend de macOS |
| 21 | Dock propio (plugin, solo Windows) |

**v0.1 pública = ítems 8 a 13, solo Windows.** Linux y macOS no se prometen hasta que existan.

## 8. Veredictos sobre ideas del dueño

| Idea | Veredicto |
|---|---|
| Personalización visual y de comportamiento de la barra | Core |
| Idiomas | Core acotado (es/en) |
| Música | Plugin (Windows primero; la API de medios del sistema es WinRT y probablemente requiera una dependencia pesada, que queda aislada en el plugin) |
| Contenedores de trabajo / modos de foco | Later (primera versión: timer de foco como plugin; bloquear apps o sitios es invasivo) |
| Barra de tareas propia | Later, proceso aparte, solo Windows |
| Menú de configuración | Ejecutable separado |
| Plugins de Git, IA, N8N, Office y avatar animado | Plugins, sin priorizar todavía |

Un chat separado de "ideas" mantiene `docs/IDEAS.md`; uno de "marketing e imagen" mantiene `docs/BRAND.md`. Son fuentes que conviene leer antes de proponer funciones o textos públicos.

## 9. Lecciones y trampas ya encontradas

- En Windows, `cargo` necesita Rust (rustup) y las Visual Studio Build Tools con la carga de trabajo "Desarrollo de escritorio con C++". Sin ella, el enlazado falla con `LNK1104: no se puede abrir msvcrt.lib`.
- Código de Windows se puede verificar desde Linux con `cargo clippy --workspace --all-targets --target x86_64-pc-windows-msvc -- -D warnings` (comprueba tipos y lints sin enlazar). Lo visual solo se confirma ejecutando en Windows.
- `as_chunks` exige Rust 1.88; por eso `rust-version = "1.88"`.
- Con `clippy.toml` los tests pueden usar `expect`/`unwrap`; el código de producción no.
- `WS_EX_NOACTIVATE` + `WS_EX_TOOLWINDOW`: la isla no roba el foco ni aparece en la barra de tareas. Se cierra con clic derecho.
- El hover usa `TrackMouseEvent(TME_LEAVE)` para saber cuándo sale el mouse; si la animación se interrumpe, continúa desde el tamaño actual.

## 10. Verificación de cualquier cambio

```
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run          # en Windows, para lo visual
```

Para medir consumo con la isla abierta (PowerShell):

```
Get-Process atoll | Select-Object @{n='RAM_MB';e={[math]::Round($_.WorkingSet64/1MB,1)}}, CPU
```

Correrlo dos veces con unos segundos de diferencia: la RAM debe mantenerse bajo 20 MB y `CPU` no debe moverse en reposo.

## 11. Reglas de honestidad para textos públicos

Solo se afirma lo medido o funcionando. Hoy: solo Windows. Linux, macOS, plugins y menú de configuración son roadmap. Todo número publicado tiene que poder reproducirse.

## 12. Estilo de trabajo con el dueño

Español rioplatense, directo. Una sola recomendación, sin menús de opciones. Archivos completos, no parches. Si algo contradice las reglas innegociables, se marca antes de avanzar.
