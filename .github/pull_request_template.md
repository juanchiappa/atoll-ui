## Resumen

<!-- Qué cambia este Pull Request y a qué ítem de docs/ROADMAP.md corresponde. -->

- **Ítem del roadmap:** 

## Checklist antes de pedir revisión

- [ ] Este PR aborda un único ítem del roadmap o un cambio atómico.
- [ ] `cargo fmt --all --check` pasa sin diferencias.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` pasa sin advertencias.
- [ ] `cargo test --workspace` pasa en verde.
- [ ] Sin `unwrap()`, `expect()` ni `panic!` en código de producción; todos los `pub` tienen documentación en español (`///`); cualquier `unsafe` está solo en `atoll-platform` con comentario `// SAFETY:`.
- [ ] Si cambió la arquitectura, el formato de configuración o el roadmap, actualicé `docs/ARCHITECTURE.md`, `docs/CONFIG.md` o `docs/ROADMAP.md` en este mismo PR.
- [ ] No agregué dependencias nuevas (o, si se aprobaron previamente, están justificadas en `docs/DEPENDENCIES.md` en este mismo PR y no son GPL/AGPL).
- [ ] Todos los commits llevan firma DCO (`git commit -s` / línea `Signed-off-by:`).

## Verificación manual

### Qué probé a mano
<!-- Comandos corridos, pruebas visuales en pantalla y medición de RAM/CPU en reposo si se tocó ventana, render o timers. -->

### Qué NO pude verificar
<!-- Qué quedó fuera de la prueba manual local y por qué. -->
