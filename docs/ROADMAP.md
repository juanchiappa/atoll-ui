# Roadmap de Atoll

Fuente de verdad del backlog: los demás chats la leen desde el repo. Cada ítem es un PR separado
(rama -> PR -> 3 checks -> Squash and merge). Los cambios de arquitectura o dependencias actualizan
`docs/ARCHITECTURE.md` y `docs/DEPENDENCIES.md` en el mismo PR.

Las ideas todavía sin decidir viven en `docs/IDEAS.md`, no acá.

## Reglas innegociables

1. **Recursos:** < 20 MB de RAM y 0 % de CPU en reposo. Loop 100 % orientado a eventos, sin render loop.
   Los timers solo existen mientras dura una animación. Única excepción (ítem 9): un timer de un solo
   disparo por minuto para el reloj.
2. **Core en Rust puro.** Sin webview ni frameworks de UI. Cada crate nuevo se justifica en
   `docs/DEPENDENCIES.md`, en el mismo PR que lo agrega.
3. **Monorepo:** `core/`, `packages/`, `plugins/`, `docs/`, `.github/`.
4. **Integraciones** (Git, IA, N8N, Office, avatar) solo como plugins en `/plugins`: procesos separados
   con IPC local. Sin plugins, el Core no carga nada extra.
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
| 9 | Reloj en la isla | En revisión |
| 10 | Escalado DPI por monitor | Pendiente |
| 11 | Protocolo IPC y host de plugins | Pendiente |
| 12 | Backend de Linux (Wayland primero) | Pendiente |
| 13 | Backend de macOS | Pendiente |

La numeración de los ítems 1 a 6 se reconstruyó desde el historial de git. Si la original difiere, se corrige acá.

## Mediciones de referencia

- Windows, build debug, isla abierta en reposo: 7,0 MB de RAM y 0,00 de CPU (hasta el ítem 5).
- Cada PR que toque render, ventana o timers tiene que volver a medir y reportar RAM y CPU en reposo.

## Ítems pendientes

### 8. Primitiva de texto en `atoll-render` (hecho)

- Solo `atoll-render`: no toca la ventana ni el backend.
- `TextRenderer::draw` dibuja un texto sobre un `Frame`; `measure` y `line_metrics` ayudan a centrarlo.
- Dependencia: `fontdue` sin features por defecto. Fuente Inter Medium recortada, embebida con `include_bytes!`
  (`docs/FONTS.md`, `THIRD_PARTY_NOTICES.md`).
- Caché de glifos acotada: lo que no cambia no se rasteriza de nuevo.
- **Aceptación:** tests de píxeles (cobertura, color, recorte, caché), fmt, clippy y tests en verde en los 3 SO,
  RAM en reposo en Windows <= 8 MB.

### 9. Reloj en la isla (en revisión)

- La hora local sale de `GetLocalTime` (Windows). Sin crates de fechas.
- El backend recibe un `Painter` (trait en `atoll-platform`) y el Core pinta el contenido (`IslandPainter`).
- En reposo la isla muestra `HH:MM`; con hover muestra la hora y la fecha.
- **Excepción deliberada a la regla de timers:** UN timer de un solo disparo por minuto, alineado al cambio de
  minuto. Se calcula cuántos milisegundos faltan y se re-arma en cada disparo. Sin polling. Solo se redibuja si
  el texto cambió. Se documenta en `docs/ARCHITECTURE.md` en el mismo PR.
- Re-armar el timer también ante `WM_TIMECHANGE` y al volver de suspensión, para no quedar desfasado.
- La animación de hover tiene que seguir funcionando con el texto adentro.
- **Aceptación:** CPU en reposo 0,00 medido; la hora cambia exactamente al minuto; RAM <= 8 MB.

### 10. Escalado DPI por monitor

- Per-Monitor-V2 (`WM_DPICHANGED`): recalcular tamaño de isla y de fuente, e invalidar la caché de glifos
  al cambiar el factor.
- **Aceptación:** misma isla, mismo tamaño físico y nítida en dos monitores con DPI distinto; sin render loop.

### 11. Protocolo IPC y host de plugins

- Protocolo en `atoll-ipc`, SDK en `atoll-plugin-sdk`, host en `atoll-core`. Named pipes en Windows y sockets
  Unix en Linux/macOS. Plugins en `/plugins/<nombre>/plugin.toml`.
- Un ADR en `docs/` fija transporte, framing, versionado y permisos antes de escribir código.
- Antes del primer plugin hay que revisar el `.gitignore` (`/plugins/*` ignora todo menos `.gitkeep`) para que los
  plugins oficiales y de ejemplo queden versionados.
- **Aceptación:** un plugin de ejemplo pinta texto en la isla; con 0 plugins la RAM no cambia.

### 12. Backend de Linux (Wayland primero)

- Isla con alfa por píxel en Wayland (layer-shell). ADR de la dependencia antes de código. X11 queda fuera.
- **Aceptación:** isla visible en compositores con layer-shell (Sway, Hyprland, KDE), 0 % de CPU en reposo;
  límites en GNOME/Mutter documentados.

### 13. Backend de macOS

- Panel sin bordes, flotante, sin ícono en el Dock. Binding mínimo a AppKit.
- **Aceptación:** isla visible sobre otras apps, 0 % de CPU en reposo.
