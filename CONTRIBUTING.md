# Guía para contribuir a Atoll

Gracias por tu interés en contribuir a Atoll. Antes de abrir un Pull Request, leé esta guía y [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) más [`docs/ROADMAP.md`](docs/ROADMAP.md).

## 1. Requisitos, clonado y compilación

Necesitás Rust **1.88** o superior (`rustup` con `rustfmt` y `clippy`). Hoy la ventana de la isla funciona en **Windows**; los crates y módulos portables (`atoll-render`, `atoll-config`, `atoll-ipc`, `atoll-plugin-sdk` y los submódulos portables de `atoll-platform`) compilan y corren sus tests en Windows, Linux y macOS.

```powershell
git clone https://github.com/juanchiappa/atoll-ui.git
cd atoll-ui
cargo test --workspace
# En Windows, para abrir la isla:
cargo run -p atoll-core
```

## 2. Reglas innegociables del proyecto

Todo cambio debe respetar estas cinco reglas:

1. **Recursos:** menos de **20 MB de RAM** y **0 % de CPU en reposo**. El loop es 100 % orientado a eventos, sin render loop ni polling, y en reposo no queda ningún timer de animación ni de hover activo. Se redibuja únicamente cuando algo cambió.
2. **Core en Rust puro:** sin webview ni frameworks de UI (`winit`, `egui`, `Tauri`, `Electron`). **No se aceptan dependencias nuevas** sin justificación escrita en [`docs/DEPENDENCIES.md`](docs/DEPENDENCIES.md) en el mismo PR y aprobación previa del mantenedor.
3. **Monorepo:** `core/`, `packages/`, `plugins/`, `docs/`, `.github/`. Todo código dependiente del SO vive exclusivamente en `core/atoll-platform` detrás de `#[cfg(target_os = "...")]`; todo bloque `unsafe` va solo en `atoll-platform`, reducido al mínimo y con comentario `// SAFETY:`.
4. **Integraciones fuera del Core:** Git, IA, N8N, Office, música, dock y cualquier otra integración van únicamente como plugins en `plugins/` (procesos separados por IPC local que envían un árbol de widgets para que el Core lo dibuje).
5. **Calidad y licencias:** cero `unwrap()`, `expect()` o `panic!` en código de producción (permitidos solo en tests), comentarios y documentación (`///`) en español en todos los ítems `pub`, identificadores en inglés, y cero dependencias GPL/AGPL.

## 3. Flujo de trabajo

1. Partí siempre de `main` limpio y actualizado en una rama por cambio atómico (`feat/...`, `fix/...`, `docs/...`, `chore/...`).
2. Usá commits convencionales en español (`feat:`, `fix:`, `docs:`, `chore:`, `refactor:`, `test:`) y **firmá cada commit con DCO (`git commit -s`)**.
3. Todo cambio de arquitectura o dependencias debe actualizar [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) o [`docs/DEPENDENCIES.md`](docs/DEPENDENCIES.md) en el mismo PR.
4. No modifiques `LICENSE` ni `TRADEMARK.md`, y nunca commitees `target/`, binarios ni videos.
5. Flujo de integración: **rama → Pull Request → los 3 checks de CI en verde (`ubuntu-latest`, `windows-latest`, `macos-latest`) → Squash and merge**.

## 4. Qué verificar antes de abrir un Pull Request

Corré y asegurate de que pasen sin errores ni advertencias:

```powershell
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Si tocaste la ventana, el render o el backend de Windows, ejecutá además `cargo run` y probá manualmente en pantalla (hover, recarga de `config.ini`, clic derecho para cerrar y medición de RAM y CPU en reposo). Detallá en la descripción del PR qué probaste a mano y qué no pudiste verificar.

## 5. Licencia de las contribuciones y firma DCO (`git commit -s`)

Todas las contribuciones enviadas a este repositorio se licencian bajo la misma licencia del proyecto: **[Apache License 2.0](LICENSE)**.

Cada commit debe incluir una línea `Signed-off-by:` generada con `git commit -s`:

```powershell
git commit -s -m "feat: descripción del cambio"
```

Eso agrega al pie del mensaje del commit:

```text
Signed-off-by: Tu Nombre <tu@email.com>
```

Con esa firma certificás que tenés derecho a enviar la contribución bajo la licencia abierta del proyecto según el **Developer Certificate of Origin (DCO) v1.1** ([developercertificate.org](https://developercertificate.org/)), cuyo texto completo se reproduce a continuación:

```text
Developer Certificate of Origin
Version 1.1

Copyright (C) 2004, 2006 The Linux Foundation and its contributors.

Everyone is permitted to copy and distribute verbatim copies of this
license document, but changing it is not allowed.


Developer's Certificate of Origin 1.1

By making a contribution to this project, I certify that:

(a) The contribution was created in whole or in part by me and I
    have the right to submit it under the open source license
    indicated in the file; or

(b) The contribution is based upon previous work that, to the best
    of my knowledge, is covered under an appropriate open source
    license and I have the right under that license to submit that
    work with modifications, whether created in whole or in part
    by me, under the same open source license (unless I am
    permitted to submit under a different license), as indicated
    in the file; or

(c) The contribution was provided directly to me by some other
    person who certified (a), (b) or (c) and I have not modified
    it.

(d) I understand and agree that this project and the contribution
    are public and that a record of the contribution (including all
    personal information I submit with it, including my sign-off) is
    maintained indefinitely and may be redistributed consistent with
    this project or the open source license(s) involved.
```
