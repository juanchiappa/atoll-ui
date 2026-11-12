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

/// Radios de las cuatro esquinas de la isla, en píxeles físicos.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CornerRadii {
    /// Radio de la esquina superior izquierda.
    pub top_left: f32,
    /// Radio de la esquina superior derecha.
    pub top_right: f32,
    /// Radio de la esquina inferior derecha.
    pub bottom_right: f32,
    /// Radio de la esquina inferior izquierda.
    pub bottom_left: f32,
}

impl CornerRadii {
    /// Las cuatro esquinas rectas (`0.0`).
    pub const ZERO: Self = Self::uniform(0.0);

    /// Asigna el mismo radio `radius` a las cuatro esquinas (estilo `island`).
    #[must_use]
    pub const fn uniform(radius: f32) -> Self {
        Self {
            top_left: radius,
            top_right: radius,
            bottom_right: radius,
            bottom_left: radius,
        }
    }

    /// Esquinas superiores rectas (`0.0`) y esquinas inferiores con radio `radius` (estilo `notch`).
    #[must_use]
    pub const fn bottom_only(radius: f32) -> Self {
        Self {
            top_left: 0.0,
            top_right: 0.0,
            bottom_right: radius,
            bottom_left: radius,
        }
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

/// Dibuja la isla por defecto: una píldora negra opaca con bordes suavizados.
///
/// # Errors
///
/// Devuelve [`RenderError::InvalidSize`] si el ancho o el alto son cero, o
/// [`RenderError::InvalidShape`] si no se puede construir la forma.
pub fn render_island(size: Size) -> Result<Frame, RenderError> {
    render_island_with(size, Color::BLACK, None)
}

/// Dibuja la isla con el color de fondo (incluyendo su alfa) y el radio uniforme dado.
///
/// Si `corner_radius` es `None`, usa forma de píldora (`min(ancho, alto) / 2`).
/// Si es `Some(r)`, el radio en píxeles físicos se acota a `0.0..=(min(ancho, alto) / 2)`.
///
/// # Errors
///
/// Devuelve [`RenderError::InvalidSize`] si el ancho o el alto son cero, o
/// [`RenderError::InvalidShape`] si no se puede construir la forma.
pub fn render_island_with(
    size: Size,
    color: Color,
    corner_radius: Option<f32>,
) -> Result<Frame, RenderError> {
    let max_radius = (size.width.min(size.height) as f32) / 2.0;
    let r = corner_radius.unwrap_or(max_radius);
    render_island_styled(size, color, CornerRadii::uniform(r))
}

/// Dibuja la isla o muesca con el color de fondo y los radios por esquina dados.
///
/// Cada esquina de `radii` se acota a `0.0..=(min(ancho, alto) / 2)`. El [`Frame`]
/// resultante tiene alfa premultiplicado listo para entregar al SO.
///
/// # Errors
///
/// Devuelve [`RenderError::InvalidSize`] si el ancho o el alto son cero, o
/// [`RenderError::InvalidShape`] si no se puede construir la forma.
pub fn render_island_styled(
    size: Size,
    color: Color,
    radii: CornerRadii,
) -> Result<Frame, RenderError> {
    let mut pixmap = Pixmap::new(size.width, size.height).ok_or(RenderError::InvalidSize(size))?;

    let width = size.width as f32;
    let height = size.height as f32;
    let path = rounded_rect(width, height, radii).ok_or(RenderError::InvalidShape)?;

    if color.a > 0 {
        let mut paint = Paint::default();
        paint.set_color_rgba8(color.r, color.g, color.b, color.a);
        paint.anti_alias = true;

        pixmap.fill_path(
            &path,
            &paint,
            FillRule::Winding,
            Transform::identity(),
            None,
        );
    }

    Ok(Frame {
        size,
        rgba: pixmap.take(),
    })
}

/// Acota un radio de esquina a `0.0..=max_radius`.
fn clamp_radius(r: f32, max_radius: f32) -> f32 {
    if r.is_finite() {
        r.clamp(0.0, max_radius)
    } else {
        max_radius
    }
}

/// Rectángulo con radio independiente en cada esquina.
fn rounded_rect(width: f32, height: f32, radii: CornerRadii) -> Option<Path> {
    /// Constante para aproximar un cuarto de círculo con una curva cúbica.
    const KAPPA: f32 = 0.552_284_7;

    let max_radius = width.min(height) / 2.0;
    let tl = clamp_radius(radii.top_left, max_radius);
    let tr = clamp_radius(radii.top_right, max_radius);
    let br = clamp_radius(radii.bottom_right, max_radius);
    let bl = clamp_radius(radii.bottom_left, max_radius);

    let mut builder = PathBuilder::new();
    builder.move_to(tl, 0.0);
    builder.line_to(width - tr, 0.0);
    if tr > 0.0 {
        let k = tr * KAPPA;
        builder.cubic_to(width - tr + k, 0.0, width, tr - k, width, tr);
    }

    builder.line_to(width, height - br);
    if br > 0.0 {
        let k = br * KAPPA;
        builder.cubic_to(
            width,
            height - br + k,
            width - br + k,
            height,
            width - br,
            height,
        );
    }

    builder.line_to(bl, height);
    if bl > 0.0 {
        let k = bl * KAPPA;
        builder.cubic_to(bl - k, height, 0.0, height - bl + k, 0.0, height - bl);
    }

    builder.line_to(0.0, tl);
    if tl > 0.0 {
        let k = tl * KAPPA;
        builder.cubic_to(0.0, tl - k, tl - k, 0.0, tl, 0.0);
    }

    builder.close();
    builder.finish()
}

#[cfg(test)]
mod tests {
    use super::{
        render_island, render_island_styled, render_island_with, Color, CornerRadii, Frame,
        RenderError, Size,
    };

    /// Píxel RGBA premultiplicado en (x, y).
    fn pixel(frame: &Frame, x: u32, y: u32) -> [u8; 4] {
        let index = ((y * frame.size().width + x) * 4) as usize;
        let px = &frame.rgba_premultiplied()[index..index + 4];
        [px[0], px[1], px[2], px[3]]
    }

    /// Alfa del píxel (x, y) de un frame.
    fn alpha(frame: &Frame, x: u32, y: u32) -> u8 {
        pixel(frame, x, y)[3]
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
    fn custom_color_and_opacity_produce_valid_premultiplied_alpha() {
        let color = Color::rgba(200, 100, 50, 128);
        let frame = render_island_with(Size::new(100, 40), color, None).expect("render válido");
        let center = pixel(&frame, 50, 20);
        assert_eq!(center[3], 128);
        assert!((i16::from(center[0]) - 100).abs() <= 1, "{center:?}");
        assert!((i16::from(center[1]) - 50).abs() <= 1, "{center:?}");
        assert!((i16::from(center[2]) - 25).abs() <= 1, "{center:?}");

        let (pixels, _) = frame.rgba_premultiplied().as_chunks::<4>();
        for &[r, g, b, a] in pixels {
            assert!(
                r <= a && g <= a && b <= a,
                "canal mayor que alfa: {r},{g},{b},{a}"
            );
        }
    }

    #[test]
    fn zero_alpha_island_is_completely_transparent() {
        let frame = render_island_with(Size::new(100, 40), Color::rgba(255, 255, 255, 0), None)
            .expect("render válido");
        assert!(frame.rgba_premultiplied().iter().all(|&b| b == 0));
    }

    #[test]
    fn custom_corner_radius_changes_corner_coverage() {
        let sharp =
            render_island_with(Size::new(100, 40), Color::WHITE, Some(0.0)).expect("rectángulo");
        assert_eq!(
            alpha(&sharp, 0, 0),
            255,
            "con radio 0 la esquina está llena"
        );

        let rounded =
            render_island_with(Size::new(100, 40), Color::WHITE, Some(10.0)).expect("radio 10");
        assert_eq!(
            alpha(&rounded, 0, 0),
            0,
            "con radio 10 la esquina(0,0) queda afuera"
        );
        assert_eq!(
            alpha(&rounded, 8, 8),
            255,
            "con radio 10 el punto (8,8) ya está adentro"
        );
    }

    #[test]
    fn notch_style_has_opaque_top_corners_and_rounded_transparent_bottom_corners() {
        let frame = render_island_styled(
            Size::new(400, 40),
            Color::BLACK,
            CornerRadii::bottom_only(20.0),
        )
        .expect("notch válido");

        // Esquinas superiores rectas: opacas.
        assert_eq!(alpha(&frame, 0, 0), 255, "esquina superior izquierda opaca");
        assert_eq!(alpha(&frame, 399, 0), 255, "esquina superior derecha opaca");

        // Esquinas inferiores redondeadas: transparentes en el extremo y opacas adentro.
        assert_eq!(
            alpha(&frame, 0, 39),
            0,
            "esquina inferior izquierda transparente"
        );
        assert_eq!(
            alpha(&frame, 399, 39),
            0,
            "esquina inferior derecha transparente"
        );
        assert_eq!(alpha(&frame, 200, 20), 255, "centro opaco");
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
