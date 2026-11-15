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

Mediciones sobre el ejecutable descargado de cada Release, en **Windows x64** (solo Windows), con la isla en reposo (sin mover el mouse):

| | [`v0.1.0`](https://github.com/juanchiappa/atoll-ui/releases/tag/v0.1.0) | [`v0.2.0`](https://github.com/juanchiappa/atoll-ui/releases/tag/v0.2.0) |
|---|---|---|
| Ejecutable `atoll.exe` | 548 KB | 691 KB |
| Zip del Release | 263 KB | 345 KB |
| RAM con la isla contraída (`WorkingSet64`) | 8,5 MB | entre 7,7 y 7,9 MB |
| RAM con la isla expandida (`WorkingSet64`) | no medido | entre 10,0 y 10,3 MB |
| Memoria según el Administrador de Tareas | no medido | no medido |
| CPU en reposo (tiempo de CPU acumulado) | no cambió entre dos mediciones separadas por 30 s | contraída: 0,016 s y expandida: 0,078 s, sin cambio en 5 minutos |

**Cómo leer estos números**

- **CPU:** en reposo no hay consumo medible: en 5 minutos con la isla quieta, contraída o expandida, el tiempo de CPU acumulado no cambió. (Para la `v0.1.0` la medición fue más corta: dos muestras separadas por 30 segundos.)
- **Memoria:** hay dos métricas distintas. `WorkingSet64` (la que usa el script de abajo) incluye páginas compartidas con el sistema; el Administrador de Tareas muestra el conjunto de trabajo privado, que suele ser menor. Por eso no se pueden comparar entre sí y se informan por separado.
- **Expandida vs. contraída:** la isla expandida usa más memoria que la contraída.
- **Tamaños:** están en KB de 1024 bytes, tal como los muestra el Explorador de Windows.
- **Alcance:** las cifras de la `v0.2.0` salen de una PC concreta (Microsoft Windows 11 Pro, compilación 26200), con 6 muestras separadas por 60 segundos por cada estado de la isla, y las de la `v0.1.0` de la medición hecha en su momento; pueden variar levemente en otras PC y versiones de Windows.

Para reproducir la medición de 5 minutos en Windows con PowerShell, con `atoll.exe` abierto y la isla quieta (el mouse sin moverse):

```powershell
$p = Get-Process atoll
1..6 | ForEach-Object {
  $p.Refresh()
  "{0:HH:mm:ss}  CPU={1:N3}s  RAM={2:N1}MB" -f (Get-Date), $p.TotalProcessorTime.TotalSeconds, ($p.WorkingSet64/1MB)
  Start-Sleep 60
}
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
