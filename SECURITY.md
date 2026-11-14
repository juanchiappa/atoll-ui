# Política de seguridad

## Versiones con soporte

Solo la **última versión publicada** de Atoll recibe correcciones de seguridad.

| Versión | Soportada |
|---|---|
| `0.1.x` (última) | Sí |
| Anteriores | No |

## Cómo reportar una vulnerabilidad

**No abras un issue público** para reportar un problema de seguridad.

Usá el canal privado de GitHub Security Advisories:

1. Entrá a la pestaña **[Security](https://github.com/juanchiappa/atoll-ui/security)** del repositorio en GitHub.
2. Hacé clic en **"Report a vulnerability"** ([nuevo reporte privado](https://github.com/juanchiappa/atoll-ui/security/advisories/new)).
3. Incluí en el reporte:
   - Versión de Atoll y versión de Windows donde lo probaste.
   - Descripción clara del problema y de su impacto real (por ejemplo, en el parser de `config.ini`, el manejo de rutas o llamadas Win32).
   - Pasos mínimos o archivo de entrada para reproducirlo.

## Tiempos de respuesta orientativos

Atoll es un proyecto de código abierto mantenido por una sola persona en su tiempo libre. Los tiempos siguientes son orientativos y de buena fe, no un SLA comercial:

- **Acuse de recibo inicial:** dentro de **7 días**.
- **Evaluación y confirmación del reporte:** dentro de **14 días**.
- **Corrección y publicación de nueva versión:** según la severidad y complejidad del fallo (normalmente entre **2 y 4 semanas** desde su confirmación), coordinando la divulgación pública una vez publicado el parche.
