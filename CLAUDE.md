# Atoll — guía para el agente

Atoll es una UI liviana y nativa (isla dinámica y más) para Windows, Linux y macOS, escrita en Rust.

## Reglas
- Presupuesto: <20 MB RAM, 0% CPU en reposo. Nunca agregues un loop que no duerma.
- Rust puro. No agregues dependencias sin que la tarea lo pida explícitamente.
- Los plugins viven en `/plugins` como procesos separados. El Core no los importa.
- Sin `unwrap()`/`expect()` fuera de tests. Errores con `Result` y tipos propios.
- Código específico de SO solo dentro de `core/atoll-platform`, con `#[cfg(target_os = "...")]`.

## Estructura
core/atoll-core · core/atoll-platform · core/atoll-render
packages/atoll-ipc · packages/atoll-plugin-sdk · plugins/ · docs/

## Antes de terminar CUALQUIER tarea
1. `cargo fmt`
2. `cargo clippy --all-targets -- -D warnings`
3. `cargo test`
Si alguno falla, arreglalo. No declares la tarea terminada con errores.

## Cómo trabajar
- Hacé solo lo que pide la tarea. No refactorices ni crees archivos extra.
- Si la tarea es ambigua, preguntá antes de escribir código.
