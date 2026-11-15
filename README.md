# Atoll

Una UI liviana y nativa para tu sistema operativo. Isla dinámica, personalización total y casi cero consumo.

- **Objetivo de recursos:** < 20 MB de RAM y 0 % de CPU en reposo (loop 100 % orientado a eventos, sin render loop ni polling)
- **Core en Rust puro**, sin webview ni frameworks pesados
- **Todo lo avanzado es opcional**: Git, IA, N8N, Office, música y dock viven como plugins separados en `/plugins`

## Estado

- **Versión actual:** `v0.2.0` (**solo Windows x64**).
- **Qué incluye hoy:** isla dinámica con transparencia alfa por píxel, reloj (`HH:MM`) y, al pasar el mouse, hora, fecha y botones de Ajustes y Cerrar (con el toolkit `atoll-ui`), escalado DPI por monitor (`Per-Monitor-V2`), archivo de configuración y temas (`%APPDATA%\Atoll\config.ini`) con recarga en vivo (`black`, `graphite`, `white`, colores, opacidad, radio de esquina) y comportamiento configurable (`hover_delay_ms`, `animation_ms`, estilo `island` o `notch`). El detalle de cada versión está en el [CHANGELOG](CHANGELOG.md).
- **Linux y macOS:** en el [roadmap](docs/ROADMAP.md) (todavía no soportados).

## Medición

> **Estos números corresponden a la versión `v0.1.0`.** La `v0.2.0` incluye más funciones (`atoll-ui`, botones de Ajustes y Cerrar, fuente ampliada) y sus mediciones se publican aparte.

Datos medidos sobre el ejecutable descargado del release [`v0.1.0`](https://github.com/juanchiappa/atoll-ui/releases/tag/v0.1.0) en **Windows x64** con la isla abierta en reposo:

- **RAM en reposo:** **8,5 MB** (`WorkingSet64`, medido vía `Get-Process`).
- **CPU en reposo:** **0 %** — el tiempo de CPU acumulado no cambia entre dos mediciones separadas por 30 segundos.
- **Tamaño:** ejecutable `atoll.exe` de **548 KB** y paquete `atoll-v0.1.0-windows-x64.zip` de **263 KB**.

Podés reproducir la medición en Windows con PowerShell mienras `atoll.exe` está abierto en reposo:

```powershell
Get-Process atoll | Select-Object Name, Id, @{N='RAM_MB';E={[math]::Round($_.WorkingSet64 / 1MB, 2)}}, CPU
Start-Sleep -Seconds 30
Get-Process atoll | Select-Object Name, Id, @{N='RAM_MB';E={[math]::Round($_.WorkingSet64 / 1MB, 2)}}, CPU
```

## Descarga

1. Descargá `atoll-v0.2.0-windows-x64.zip` (la última versión) desde [Releases → Latest](https://github.com/juanchiappa/atoll-ui/releases/latest). Para ver o descargar versiones anteriores (por ejemplo `v0.1.0`), entrá a la sección de [Releases](https://github.com/juanchiappa/atoll-ui/releases) y elegí la que quieras.
2. *(Opcional)* Verificá la integridad del archivo contra `atoll-v0.2.0-windows-x64.zip.sha256` en PowerShell:
   ```powershell
   Get-FileHash .\atoll-v0.2.0-windows-x64.zip -Algorithm SHA256
   ```
3. Descomprimí el `.zip` en cualquier carpeta y abrí `atoll.exe` (no requiere instalador).
   - **Nota sobre Windows SmartScreen:** como el ejecutable todavía no está firmado digitalmente, Windows puede mostrar la advertencia *"Windows protegió su PC"* la primera vez. Hacé clic en **Más información** y luego en **Ejecutar de todas formas**.
4. **Uso y configuración:**
   - Pasá el mouse por encima de la isla para expandirla y ver la hora y la fecha. Al terminar de expandirse aparecen los botones de **Ajustes** (abre `config.ini`) y **Cerrar**.
   - Hacé **clic derecho** sobre la isla para cerrarla.
   - Al ejecutarse por primera vez se crea `%APPDATA%\Atoll\config.ini`. Editá ese archivo con cualquier editor de texto y guardá: los cambios de tamaño, tema, colores y comportamiento se aplican en vivo sin reiniciar. Consultá todas las opciones en [`docs/CONFIG.md`](docs/CONFIG.md).

## Estructura

| Carpeta | Contenido |
|---|---|
| `core/` | Binario `atoll`, capa de plataforma (`atoll-platform`) y render por software (`atoll-render`) |
| `packages/` | Configuración y temas (`atoll-config`), protocolo IPC (`atoll-ipc`) y SDK de plugins (`atoll-plugin-sdk`) |
| `plugins/` | Plugins oficiales (procesos independientes) |
| `docs/` | Arquitectura, configuración, dependencias, fuentes, guía de releases y roadmap |

## Contribuir y seguridad

- Guía para colaboradores: [CONTRIBUTING.md](CONTRIBUTING.md).
- Reporte privado de vulnerabilidades: [SECURITY.md](SECURITY.md).

## Licencia

Open source bajo [Apache License 2.0](LICENSE). El nombre y el logo de Atoll
no están incluidos en la licencia: ver [TRADEMARK.md](TRADEMARK.md).
