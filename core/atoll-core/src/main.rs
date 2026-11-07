//! Punto de entrada del Core de Atoll.

mod island;

use std::error::Error;
use std::process::ExitCode;

use atoll_platform::{native_backend, IslandConfig};
use atoll_render::Size;

use crate::island::IslandPainter;

/// Tamaño de la isla en reposo.
const ISLAND_SIZE: Size = Size::new(400, 40);

/// Tamaño de la isla cuando el mouse está encima.
const ISLAND_HOVER_SIZE: Size = Size::new(480, 64);

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
fn run() -> Result<(), Box<dyn Error>> {
    let config = IslandConfig::new(ISLAND_SIZE, ISLAND_HOVER_SIZE, TOP_MARGIN);
    let mut backend = native_backend()?;
    let painter = IslandPainter::new(ISLAND_SIZE, ISLAND_HOVER_SIZE)?;
    backend.run(&config, Box::new(painter))?;
    Ok(())
}
