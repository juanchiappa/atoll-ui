# Dependencias externas

Regla del proyecto: cada crate nuevo se justifica por escrito acá, en el mismo PR que lo agrega.

## `windows-sys` `0.59` (solo Windows, solo `atoll-platform`)

- **Qué es:** bindings crudos a la API Win32, generados por Microsoft. Sin runtime, sin código propio más allá de las declaraciones.
- **Por qué:** crear la ventana de la isla y recibir eventos requiere Win32. Es la opción más liviana: el crate hermano `windows` agrega capas y tiempos de compilación, y los frameworks de UI (winit, egui) agregan decenas de dependencias y memoria.
- **Costo:** cero en Linux y macOS (`[target.'cfg(windows)'.dependencies]`). En Windows, solo los módulos activados por features: `Win32_Foundation`, `Win32_Graphics_Gdi`, `Win32_System_LibraryLoader`, `Win32_System_SystemInformation` (`GetLocalTime`, para la hora del reloj sin depender de crates de fechas), `Win32_UI_HiDpi` (DPI por monitor), `Win32_UI_Input_KeyboardAndMouse` (aviso de que el mouse salió de la isla) y `Win32_UI_WindowsAndMessaging` (incluye `SetCoalescableTimer` y los avisos de cambio de hora y suspensión). Las features `Win32_System_SystemInformation` (ítem 9) y `Win32_UI_HiDpi` (ítem 10: `SetProcessDpiAwarenessContext` y `GetDpiForWindow`, para el escalado por monitor) no suman ningún crate nuevo.
- **Licencia:** MIT OR Apache-2.0 (compatible con Apache-2.0).
- **Mantenedor:** Microsoft.

## `tiny-skia` `0.11` (`atoll-render`)

- **Qué es:** rasterizador 2D por software, escrito en Rust, basado en el motor de Skia. Dibuja formas con bordes suavizados (anti-aliasing) sobre un buffer en memoria.
- **Por qué:** una isla redondeada y suave no se logra con las regiones de GDI (bordes serrados). Necesitamos un rasterizador propio y portable que funcione igual en Windows, Linux y macOS sin GPU. Escribir uno desde cero sería más código para mantener y más bugs que depender de este.
- **Configuración mínima:** `default-features = false, features = ["std"]`. Sin SIMD, sin soporte PNG: se desactivan para mantener el binario chico. La puerta queda abierta a activarlos si una medición lo justifica.
- **Dependencias transitivas** (todas pequeñas, sin build scripts pesados): `tiny-skia-path`, `arrayref`, `arrayvec`, `bytemuck`, `cfg-if`, `log`, `strict-num`.
- **Licencia:** BSD-3-Clause (compatible con Apache-2.0).
- **Mantenimiento:** proyecto de la comunidad `linebender`, ampliamente usado (resvg, Iced, Slint).

## `fontdue` `0.9` (`atoll-render`)

- **Qué es:** rasterizador de glifos TrueType/OpenType en Rust puro. Convierte un carácter y un tamaño en píxeles (cobertura de 8 bits). No hace layout, no busca fuentes del sistema, no hace shaping.
- **Por qué:** la isla tiene que mostrar texto (hora y fecha). `tiny-skia` no dibuja texto. Hace falta un rasterizador de glifos que funcione igual en Windows, Linux y macOS, sin APIs del SO y sin cargar nada si no se usa.
- **Configuración mínima:** `default-features = false`. Quedan afuera `simd` (rasterizado vectorizado, innecesario para unos pocos glifos cacheados) y `hashbrown` (un HashMap propio; se usa el de `std`). `parallel`, que trae `rayon` (un pool de hilos que no queremos en un proceso que duerme), no es default y no se activa. Los glifos se rasterizan de a uno y una sola vez gracias a la caché de `atoll-render`, así que no hay nada que acelerar.
- **Dependencias transitivas (3):** `ttf-parser` (lee la fuente), `core_maths` y `libm` (matemática flotante). Cero transitivas no existe con ninguna librería de fuentes: todas traen un parser TTF. La alternativa es escribir uno propio, que son miles de líneas para mantener y una fuente de bugs.
- **Licencias:** `fontdue` MIT OR Apache-2.0 OR Zlib; `ttf-parser` MIT OR Apache-2.0; `core_maths` MIT; `libm` MIT. Todas compatibles con Apache-2.0, ninguna GPL/AGPL.
- **MSRV:** `ttf-parser` 1.63 y `libm` 1.63; `fontdue` y `core_maths` no declaran, y compilan con 1.88+ (el proyecto exige 1.88). Solo `libm` tiene `build.rs`.
- **Costo de RAM medido (Linux, build release con el perfil del workspace):** +252 KB al crear el `TextRenderer` (carga de la fuente embebida) y +4 a 8 KB al dibujar la hora y la fecha (13 glifos en caché). Total aproximado: < 0,3 MB, manteniendo el proceso en Windows dentro del presupuesto de ~7-8 MB en reposo.
- **Fuente embebida:** Inter Medium recortada a 87 glifos, 8,8 KB en el binario. Origen y licencia en `docs/FONTS.md` y `THIRD_PARTY_NOTICES.md`.

### Alternativas descartadas (comparación)

| Opción | Dependencias transitivas | RAM estimada | Motivo |
|---|---|---|---|
| **`fontdue`** (elegida) | 3 | +0,25 MB medido | Pequeña, rápida, Rust puro, sin sistema de fuentes |
| `ab_glyph` | 3 (`ab_glyph_rasterizer`, `owned_ttf_parser`, `ttf-parser`) | similar con un subset chico (parsea los glifos bajo demanda) | Mismo costo de dependencias y sin ventaja real con una fuente de 87 glifos; `fontdue` es más rápida y es la preferida del proyecto |
| `cosmic-text` | decenas (`fontdb`, `rustybuzz`, `swash`, tablas Unicode) | varios MB | Escanea fuentes del sistema y trae un motor de layout. Prohibido por la regla 1 |
| `rusttype` | 3 o más | similar | Sin mantenimiento, reemplazado por `ab_glyph` |
| GDI / DirectWrite / CoreText / FreeType | 0 crates, pero código por SO | variable | Cuatro implementaciones distintas, aspecto distinto por SO y alfa por píxel problemático con GDI |

Las cifras de `ab_glyph`, `cosmic-text` y `rusttype` son estimaciones; solo `fontdue` se midió.
