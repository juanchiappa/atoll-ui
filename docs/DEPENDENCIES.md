# Dependencias externas

Regla del proyecto: cada crate nuevo se justifica por escrito acá, en el mismo PR que lo agrega.

## `windows-sys` (solo Windows, solo `atoll-platform`)

- **Qué es:** bindings crudos a la API Win32, generados por Microsoft. Sin runtime, sin código propio más allá de las declaraciones.
- **Por qué:** crear la ventana de la isla y recibir eventos requiere Win32. Es la opción más liviana: el crate hermano `windows` agrega capas y tiempos de compilación, y los frameworks de UI (winit, egui) agregan decenas de dependencias y memoria.
- **Costo:** cero en Linux y macOS (`[target.'cfg(windows)'.dependencies]`). En Windows, solo los módulos activados por features: `Win32_Foundation`, `Win32_Graphics_Gdi`, `Win32_System_LibraryLoader`, `Win32_UI_WindowsAndMessaging`.
- **Licencia:** MIT OR Apache-2.0 (compatible con Apache-2.0).
- **Mantenedor:** Microsoft.

## `tiny-skia` (`atoll-render`)

- **Qué es:** rasterizador 2D por software, escrito en Rust, basado en el motor de Skia. Dibuja formas con bordes suavizados (anti-aliasing) sobre un buffer en memoria.
- **Por qué:** una isla redondeada y suave no se logra con las regiones de GDI (bordes serrados). Necesitamos un rasterizador propio y portable que funcione igual en Windows, Linux y macOS sin GPU. Escribir uno desde cero sería más código para mantener y más bugs que depender de este.
- **Configuración mínima:** `default-features = false, features = ["std"]`. Sin SIMD, sin soporte PNG: se desactivan para mantener el binario chico. La puerta queda abierta a activarlos si una medición lo justifica.
- **Dependencias transitivas** (todas pequeñas, sin build scripts pesados): `tiny-skia-path`, `arrayref`, `arrayvec`, `bytemuck`, `cfg-if`, `log`, `strict-num`.
- **Licencia:** BSD-3-Clause (compatible con Apache-2.0).
- **Mantenimiento:** proyecto de la comunidad `linebender`, ampliamente usado (resvg, Iced, Slint).
