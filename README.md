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
Código fuente disponible bajo [PolyForm Noncommercial 1.0.0](LICENSE). Gratis para uso personal, educativo y sin fines de lucro. El uso comercial requiere permiso. El nombre y el logo están cubiertos por [TRADEMARK.md](TRADEMARK.md).
