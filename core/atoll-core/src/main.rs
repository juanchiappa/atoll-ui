//! Punto de entrada del Core de Atoll.

use atoll_platform::Os;
use atoll_render::Size;

fn main() {
    let island = Size::new(400, 40);
    println!(
        "Atoll {} en {:?} (isla {}x{})",
        env!("CARGO_PKG_VERSION"),
        Os::current(),
        island.width,
        island.height
    );
}
