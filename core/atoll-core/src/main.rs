//! Punto de entrada del Core de Atoll.

use std::process::ExitCode;

use atoll_platform::{native_backend, IslandConfig, PlatformError};
use atoll_render::Size;

/// Margen entre el borde superior de la pantalla y la isla, en píxeles.
const TOP_MARGIN: u32 = 8;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("atoll: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Abre la isla con el backend nativo y espera a que se cierre.
fn run() -> Result<(), PlatformError> {
    let size = Size::new(400, 40);
    let config = IslandConfig::new(size.width, size.height, TOP_MARGIN);
    let mut backend = native_backend()?;
    backend.run(&config)
}
