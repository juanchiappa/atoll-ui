# Cómo publicar una versión de Atoll

Este documento describe el procedimiento paso a paso para preparar, probar, publicar y medir un release oficial de Atoll en GitHub.

## 1. Preparar la versión en una rama

1. Parate en `main` limpio y actualizado:
   ```powershell
   git checkout main
   git pull --ff-only
   git status
   ```
2. Creá una rama para la versión (por ejemplo `chore/release-v0-1-0`):
   ```powershell
   git checkout -b chore/release-v0-1-0
   ```
3. Verificá (o actualizá) la versión del workspace en `Cargo.toml` bajo `[workspace.package]`:
   ```toml
   [workspace.package]
   version = "0.1.0"
   ```
   Si cambiaste la versión, ejecutá `cargo check --workspace` para que `Cargo.lock` quede sincronizado.
4. Actualizá `CHANGELOG.md` siguiendo el formato [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/) con un encabezado exacto `## [X.Y.Z] - AAAA-MM-DD`. El workflow de release extrae automáticamente el contenido de esa sección para armar las notas del Release; si la sección no existe o está vacía, el workflow falla.
5. Corré las verificaciones locales obligatorias:
   ```powershell
   cargo fmt --all --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   ```
6. Commiteá los cambios, abrí el Pull Request hacia `main`, esperá los 3 checks de CI (`ubuntu-latest`, `windows-latest`, `macos-latest`) y mergeá con **Squash and merge**.

## 2. Probar el empaquetado sin publicar (`workflow_dispatch`)

Antes de crear el tag, probá el workflow manualmente desde GitHub Actions:

1. En GitHub, entrá a **Actions → release → Run workflow** sobre la rama `main`.
2. Esta corrida manual ejecuta únicamente el job `package` (con permisos `contents: read`):
   - Verifica `Cargo.toml` y extrae las notas de `CHANGELOG.md`.
   - Compila `atoll.exe` con `cargo build --release --locked -p atoll-core`.
   - Arma `atoll-vX.Y.Z-windows-x64.zip` (`atoll.exe`, `LICENSE`, `THIRD_PARTY_NOTICES.md`, `LEEME.txt`) y `atoll-vX.Y.Z-windows-x64.zip.sha256`.
   - Sube el artifact `atoll-vX.Y.Z-windows-x64` sin crear ningún Release público.
3. Descargá el artifact desde el resumen de la corrida, verificá el SHA256 y probá `atoll.exe` en Windows (ver sección 4).

## 3. Crear y empujar el tag (`vX.Y.Z`)

Una vez verificado el artifact de prueba:

1. Actualizá `main` localmente:
   ```powershell
   git checkout main
   git pull --ff-only
   ```
2. Creá un tag anotado `vX.Y.Z` (debe coincidir exactamente con `version` en `Cargo.toml`) y empujalo:
   ```powershell
   git tag -a v0.1.0 -m "Atoll v0.1.0"
   git push origin v0.1.0
   ```
3. Al empujar el tag `v0.1.0`, el workflow `.github/workflows/release.yml` corre automáticamente:
   - Verifica que el tag `vX.Y.Z` coincida con `[workspace.package].version` en `Cargo.toml`.
   - Extrae las notas de la sección `## [X.Y.Z]` de `CHANGELOG.md`.
   - Compila y empaqueta `atoll-vX.Y.Z-windows-x64.zip` y `atoll-vX.Y.Z-windows-x64.zip.sha256`.
   - Comprueba con `gh release view` que no exista ya un Release para ese tag (si existe, falla sin sobrescribirlo) y publica el Release con `gh release create`.

## 4. Verificar el zip y medir RAM y CPU en reposo

1. Descargá `atoll-vX.Y.Z-windows-x64.zip` y `atoll-vX.Y.Z-windows-x64.zip.sha256` desde la página de Releases de GitHub.
2. Comprobá el checksum SHA256 en PowerShell:
   ```powershell
   Get-FileHash .\atoll-v0.1.0-windows-x64.zip -Algorithm SHA256
   Get-Content .\atoll-v0.1.0-windows-x64.zip.sha256
   ```
3. Descomprimí el archivo, ejecutá `atoll.exe` y dejá la isla abierta en reposo (sin apoyar el mouse encima).
4. Medí el consumo de memoria (`WorkingSet64`) y el tiempo acumulado de CPU dos veces separadas por 30 segundos:
   ```powershell
   Get-Process atoll | Select-Object Name, Id, @{N='RAM_MB';E={[math]::Round($_.WorkingSet64 / 1MB, 2)}}, CPU
   Start-Sleep -Seconds 30
   Get-Process atoll | Select-Object Name, Id, @{N='RAM_MB';E={[math]::Round($_.WorkingSet64 / 1MB, 2)}}, CPU
   ```
   - **Criterio de aceptación:** `RAM_MB < 20` y el valor de `CPU` (segundos acumulados de procesador) idéntico entre ambas muestras (`0,00` de diferencia en 30 segundos = 0 % de CPU en reposo).
5. Cerrá la isla haciendo clic derecho sobre ella.
