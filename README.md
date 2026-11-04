# Atoll

Una UI liviana y nativa para tu sistema operativo. Isla dinámica, personalización total y casi cero consumo.

- **< 20 MB de RAM** y **0% de CPU en reposo**
- **Windows, Linux y macOS**
- **Core en Rust puro**, sin webview ni bloatware
- **Todo lo avanzado es opcional**: Git, IA, N8N, Office y más viven como plugins en `/plugins`

> Estado: en desarrollo temprano. Todavía no hay releases.

## Estructura

| Carpeta | Contenido |
|---|---|
| `core/` | Motor, capa de plataforma y render |
| `packages/` | Protocolo IPC y SDK de plugins |
| `plugins/` | Plugins opcionales |
| `docs/` | Arquitectura y decisiones |

## Licencia

Open source bajo [Apache License 2.0](LICENSE). El nombre y el logo de Atoll
no están incluidos en la licencia: ver [TRADEMARK.md](TRADEMARK.md).
