# Atoll

Una UI liviana y nativa para tu sistema operativo. Isla dinámica, personalización total y casi cero consumo.

- **Objetivo de recursos:** < 20 MB de RAM y 0 % de CPU en reposo (loop 100 % orientado a eventos, sin render loop ni polling)
- **Core en Rust puro**, sin webview ni frameworks pesados
- **Todo lo avanzado es opcional**: Git, IA, N8N, Office, música y dock viven como plugins separados en `/plugins`

## Estado

- **Versión actual:** `v0.1.0` (**solo Windows x64**).
- **Qué incluye hoy:** isla dinámica con transparencia alfa por píxel, reloj (`HH:MM`) y fecha al pasar el mouse, escalado DPI por monitor (`Per-Monitor-V2`), archivo de configuración y temas (`%APPDATA%\Atoll\config.ini`) con recarga en vivo (`black`, `graphite`, `white`, colores, opacidad, radio de esquina) y comportamiento configurable (`hover_delay_ms`, `animation_ms`, estilo `island` o `notch`).
- **Linux y macOS:** en el [roadmap](docs/ROADMAP.md) (todavía no soportados).

## Descarga

1. Descargá `atoll-v0.1.0-windows-x64.zip` desde la sección de [Releases](https://github.com/juanchiappa/atoll-ui/releases).
2. *(Opcional)* Verificá la integridad del archivo contra `atoll-v0.1.0-windows-x64.zip.sha256` en PowerShell:
   ```powershell
   Get-FileHash .\atoll-v0.1.0-windows-x64.zip -Algorithm SHA256
   ```
3. Descomprimí el `.zip` en cualquier carpeta y abrí `atoll.exe` (no requiere instalador).
   - **Nota sobre Windows SmartScreen:** como el ejecutable todavía no está firmado digitalmente, Windows puede mostrar la advertencia *"Windows protegió su PC"* la primera vez. Hacé clic en **Más información** y luego en **Ejecutar de todas formas**.
4. **Uso y configuración:**
   - Pasá el mouse por encima de la isla para expandirla y ver la fecha.
   - Hacé **clic derecho** sobre la isla para cerrarla.
   - Al ejecutarse por primera vez se crea `%APPDATA%\Atoll\config.ini`. Editá ese archivo con cualquier editor de texto y guardá: los cambios de tamaño, tema, colores y comportamiento se aplican en vivo sin reiniciar. Consultá todas las opciones en [`docs/CONFIG.md`](docs/CONFIG.md).

## Estructura

| Carpeta | Contenido |
|---|---|
| `core/` | Binario `atoll`, capa de plataforma (`atoll-platform`) y render por software (`atoll-render`) |
| `packages/` | Configuración y temas (`atoll-config`), protocolo IPC (`atoll-ipc`) y SDK de plugins (`atoll-plugin-sdk`) |
| `plugins/` | Plugins oficiales (procesos independientes) |
| `docs/` | Arquitectura, configuración, dependencias, fuentes, guía de releases y roadmap |

## Licencia

Open source bajo [Apache License 2.0](LICENSE). El nombre y el logo de Atoll
no están incluidos en la licencia: ver [TRADEMARK.md](TRADEMARK.md).
