# Atoll — reglas para agentes de código

Fuente canónica de reglas. Leelas completas antes de tocar código. El contexto largo, las decisiones y su porqué están en `docs/HANDOFF.md`. El repo es la fuente de verdad: si algo de acá contradice el código, avisá antes de seguir.

## Producto
Atoll (repo `juanchiappa/atoll-ui`) es una UI liviana y nativa para Windows, Linux y macOS. Nació como isla dinámica y apunta a personalizar el SO. Core en Rust. Licencia Apache-2.0; el nombre y el logo están protegidos por `TRADEMARK.md`. Hoy funciona solo en Windows.

## Reglas innegociables
Si una tarea contradice alguna, PARÁ y avisá antes de escribir código.
1. Recursos: menos de 20 MB de RAM y 0% de CPU en reposo (medido: ~7 MB). Loop 100% orientado a eventos, sin render loop, sin polling.
2. Core en Rust puro. Sin webview, sin frameworks de UI (nada de winit, egui, Tauri, Electron). Ninguna dependencia nueva sin justificación escrita en `docs/DEPENDENCIES.md` Y aprobación del dueño.
3. Monorepo: `core/`, `packages/`, `plugins/`, `docs/`, `.github/`.
4. Integraciones (Git, IA, N8N, Office, avatar, música, dock) SOLO como plugins en `plugins/`: procesos separados con IPC local. Sin plugins, el Core no carga nada extra. Los plugins no abren ventanas propias: mandan un árbol de widgets y el Core lo dibuja.
5. Código listo para público: sin `unwrap`/`expect`/`panic!` en producción, clippy y fmt limpios, docs en todo `pub`, sin dependencias GPL/AGPL.

## Estructura
```
core/atoll-core        binario "atoll": loop de eventos, estado, host de plugins
core/atoll-platform    TODO el código específico de SO (cfg(target_os)); módulo portable `animation`
core/atoll-render      render por software con tiny-skia; texto
packages/atoll-ipc     protocolo IPC Core <-> plugins
packages/atoll-plugin-sdk
plugins/               plugins oficiales (cada uno, proyecto Cargo independiente)
docs/                  ARCHITECTURE.md, DEPENDENCIES.md, ROADMAP.md, HANDOFF.md
```
Planeado: `packages/atoll-ui` (mini toolkit de widgets) y `core/atoll-settings` (menú de configuración, ejecutable aparte).

## Estado
Hecho (Windows): ventana de isla con alfa por píxel (`WS_EX_LAYERED` + `UpdateLayeredWindow`), render con tiny-skia (borde suavizado), animación de hover 400x40 -> 480x64 en 180 ms con timer efímero, texto, reloj y escalado DPI por monitor. Linux y macOS devuelven `Unsupported`.
Verificá el estado real leyendo `Cargo.toml` de cada crate, `docs/ROADMAP.md` y `docs/DEPENDENCIES.md`.

## Dependencias aprobadas
- `windows-sys` 0.59 (solo `cfg(windows)`, solo `atoll-platform`), con features mínimas.
- `tiny-skia` 0.11 con `default-features = false, features = ["std"]`.
- Las de texto/fuente del ítem 8: ver `docs/DEPENDENCIES.md`.
Cualquier otra está prohibida hasta que se apruebe.

## Convenciones de código
- Edición 2021, `rust-version = "1.88"`. `Cargo.lock` SE COMMITEA.
- Lints del workspace: `unwrap_used`, `expect_used`, `panic` en deny; `missing_docs` en warn. `clippy.toml` permite `unwrap`/`expect` solo en tests.
- `unsafe` solo en `atoll-platform`, en bloques mínimos, cada uno con comentario `// SAFETY:`.
- Código de SO solo en `atoll-platform`. La lógica que no depende del SO (estado, animaciones) va en módulos portables, con tests que corran en Linux.
- Errores con `Result` y tipos propios. `Box<dyn Error>` solo en `main`.
- Comentarios y docs en español; identificadores en inglés.
- Timers: solo dos permitidos. (a) El de animación, que existe únicamente mientras dura la animación. (b) Un timer de un solo disparo por minuto para el reloj, alineado al cambio de minuto. Nada de polling ni `sleep` en hilos. Cualquier otro necesita aprobación y se documenta en `docs/ARCHITECTURE.md`.
- Se redibuja solo cuando algo cambió.

## Flujo de trabajo
1. Partí de `main` actualizado, en una rama: `feat/...`, `fix/...`, `docs/...`.
2. Un ítem del backlog por rama. Cambios atómicos.
3. Antes de dar algo por terminado, corré y pasá:
   - `cargo fmt --all --check`
   - `cargo clippy --workspace --all-targets -- -D warnings`
   - `cargo test --workspace`
   - Si tocaste la ventana o el backend de Windows: `cargo run` y probá a mano (hover, clic derecho para cerrar).
4. Commits convencionales en español: `feat:`, `fix:`, `docs:`, `chore:`, `refactor:`, `test:`.
5. NUNCA hagas push a `main` ni force push. Dejá la rama commiteada y avisame: yo hago el push y el Pull Request. El CI corre en ubuntu, windows y macos; se mergea con Squash and merge.
6. Todo cambio de arquitectura o dependencias actualiza `docs/ARCHITECTURE.md` o `docs/DEPENDENCIES.md` en el mismo cambio.
7. Cierre de cada tarea: qué hiciste, qué verificaste (comandos y resultado) y qué NO pudiste verificar.

## Protegido
No modifiques `LICENSE` ni `TRADEMARK.md`. No toques `.github/workflows/` salvo que la tarea lo pida. Nunca commitees `target/`, binarios ni videos.

## Backlog (fuente: `docs/ROADMAP.md`)
11 Archivo de configuración y temas. 12 Comportamiento configurable. 13 Release v0.1. 14 `atoll-ui`. 15 IPC y host de plugins. 16 Idiomas es/en. 17 `atoll-settings`. 18 Plugin de música (Windows). 19 Backend de Linux. 20 Backend de macOS. 21 Dock (plugin, Windows).
La v0.1 pública es: isla + reloj + DPI + configuración y temas + comportamiento, solo Windows.

## Cómo hablarme
Español rioplatense, directo. Una sola recomendación, no menús de opciones. Archivos completos, no parches. Si algo es ambiguo, preguntá UNA vez antes de escribir código.
