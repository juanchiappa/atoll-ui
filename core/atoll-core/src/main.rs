//! Punto de entrada del Core de Atoll.

use std::error::Error;
use std::process::ExitCode;

use atoll_platform::{native_backend, IslandConfig};
use atoll_render::{render_island, Size};

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

/// Dibuja la isla, la abre con el backend nativo y espera a que se cierre.
fn run() -> Result<(), Box<dyn Error>> {
    let size = Size::new(400, 40);
    let frame = render_island(size)?;
    let config = IslandConfig::new(size.width, size.height, TOP_MARGIN);
    let mut backend = native_backend()?;
    backend.run(&config, &frame)?;
    Ok(())
}
