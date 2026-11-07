//! Render de Atoll.
//!
//! Se dibuja por software, solo cuando hace falta. No existe un render loop:
//! cada llamada produce un [`Frame`] y quien lo pidió decide cuándo mostrarlo.

mod text;

use std::fmt;

pub use text::{Color, LineMetrics, TextRenderer};
use tiny_skia::{FillRule, Paint, Path, PathBuilder, Pixmap, Transform};

/// Tamaño en píxeles de una superficie.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Size {
    /// Ancho en píxeles.
    pub width: u32,
    /// Alto en píxeles.
    pub height: u32,
}

impl Size {
    /// Crea un tamaño a partir de ancho y alto.
    #[must_use]
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// Cantidad total de píxeles de la superficie.
    #[must_use]
    pub const fn area(&self) -> u64 {
        self.width as u64 * self.height as u64
    }
}

/// Errores del render.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderError {
    /// El tamaño pedido es cero o demasiado grande.
    InvalidSize(Size),
    /// No se pudo construir la forma a dibujar.
    InvalidShape,
    /// La fuente embebida no se pudo cargar.
    InvalidFont,
    /// Tamaño de fuente o posición de texto fuera de rango (cero, negativo, no finito o demasiado grande).
    InvalidTextParams,
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSize(size) => {
                write!(f, "tamaño inválido: {}x{}", size.width, size.height)
            }
            Self::InvalidShape => write!(f, "no se pudo construir la forma"),
            Self::InvalidFont => write!(f, "no se pudo cargar la fuente embebida"),
            Self::InvalidTextParams => write!(f, "tamaño o posición de texto inválidos"),
        }
    }
}

impl std::error::Error for RenderError {}

/// Imagen renderizada: píxeles RGBA de 8 bits con alfa premultiplicado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    size: Size,
    rgba: Vec<u8>,
}

impl Frame {
    /// Tamaño de la imagen en píxeles.
    #[must_use]
    pub const fn size(&self) -> Size {
        self.size
    }

    /// Píxeles en orden RGBA, filas de arriba hacia abajo, alfa premultiplicado.
    #[must_use]
    pub fn rgba_premultiplied(&self) -> &[u8] {
        &self.rgba
    }

    /// Copia de los píxeles en orden BGRA (el que espera Windows).
    #[must_use]
    pub fn bgra_premultiplied(&self) -> Vec<u8> {
        let (pixels, _) = self.rgba.as_chunks::<4>();
        pixels
            .iter()
            .flat_map(|&[r, g, b, a]| [b, g, r, a])
            .collect()
    }
}

/// Dibuja la isla: una píldora negra con bordes suavizados sobre fondo transparente.
///
/// # Errors
///
/// Devuelve [`RenderError::InvalidSize`] si el ancho o el alto son cero, o
/// [`RenderError::InvalidShape`] si no se puede construir la forma.
pub fn render_island(size: Size) -> Result<Frame, RenderError> {
    let mut pixmap = Pixmap::new(size.width, size.height).ok_or(RenderError::InvalidSize(size))?;

    let width = size.width as f32;
    let height = size.height as f32;
    let radius = width.min(height) / 2.0;
    let path = rounded_rect(width, height, radius).ok_or(RenderError::InvalidShape)?;

    let mut paint = Paint::default();
    paint.set_color_rgba8(0, 0, 0, 255);
    paint.anti_alias = true;

    pixmap.fill_path(
        &path,
        &paint,
        FillRule::Winding,
        Transform::identity(),
        None,
    );

    Ok(Frame {
        size,
        rgba: pixmap.take(),
    })
}

/// Rectángulo con esquinas redondeadas; con `radius = min(w, h) / 2` es una píldora.
fn rounded_rect(width: f32, height: f32, radius: f32) -> Option<Path> {
    /// Constante para aproximar un cuarto de círculo con una curva cúbica.
    const KAPPA: f32 = 0.552_284_7;
    let k = radius * KAPPA;

    let mut builder = PathBuilder::new();
    builder.move_to(radius, 0.0);
    builder.line_to(width - radius, 0.0);
    builder.cubic_to(width - radius + k, 0.0, width, radius - k, width, radius);
    builder.line_to(width, height - radius);
    builder.cubic_to(
        width,
        height - radius + k,
        width - radius + k,
        height,
        width - radius,
        height,
    );
    builder.line_to(radius, height);
    builder.cubic_to(
        radius - k,
        height,
        0.0,
        height - radius + k,
        0.0,
        height - radius,
    );
    builder.line_to(0.0, radius);
    builder.cubic_to(0.0, radius - k, radius - k, 0.0, radius, 0.0);
    builder.close();
    builder.finish()
}

#[cfg(test)]
mod tests {
    use super::{render_island, Frame, RenderError, Size};

    /// Alfa del píxel (x, y) de un frame.
    fn alpha(frame: &Frame, x: u32, y: u32) -> u8 {
        let index = ((y * frame.size().width + x) * 4 + 3) as usize;
        frame.rgba_premultiplied()[index]
    }

    #[test]
    fn area_is_width_times_height() {
        assert_eq!(Size::new(400, 40).area(), 16_000);
    }

    #[test]
    fn island_center_is_opaque_and_corner_is_transparent() {
        let frame = render_island(Size::new(400, 40)).expect("render válido");
        assert_eq!(alpha(&frame, 200, 20), 255);
        assert_eq!(alpha(&frame, 0, 0), 0);
        assert_eq!(alpha(&frame, 399, 39), 0);
    }

    #[test]
    fn island_edges_are_antialiased() {
        let frame = render_island(Size::new(400, 40)).expect("render válido");
        let (pixels, _) = frame.rgba_premultiplied().as_chunks::<4>();
        let partial = pixels.iter().filter(|px| px[3] > 0 && px[3] < 255).count();
        assert!(partial > 0, "el borde tiene que tener píxeles parciales");
    }

    #[test]
    fn zero_size_is_rejected() {
        assert_eq!(
            render_island(Size::new(0, 40)),
            Err(RenderError::InvalidSize(Size::new(0, 40)))
        );
    }

    #[test]
    fn bgra_swaps_red_and_blue() {
        let frame = Frame {
            size: Size::new(1, 1),
            rgba: vec![10, 20, 30, 255],
        };
        assert_eq!(frame.bgra_premultiplied(), vec![30, 20, 10, 255]);
    }
}
