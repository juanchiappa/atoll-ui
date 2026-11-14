//! Render de Atoll.
//!
//! Se dibuja por software, solo cuando hace falta. No existe un render loop:
//! cada llamada produce un [`Frame`] y quien lo pidió decide cuándo mostrarlo.

mod text;

use std::fmt;

pub use text::{Color, LineMetrics, TextRenderer};
use tiny_skia::{
    FillRule, LineCap, LineJoin, Paint, Path, PathBuilder, Pixmap, PixmapMut, Stroke, Transform,
};

/// Tamaño de la grilla entera normalizada para comandos de íconos vectoriales (`24 × 24`).
pub const ICON_GRID_SIZE: i16 = 24;

/// Rectángulo entero en coordenadas de píxeles del [`Frame`], usado para posicionar y recortar (`clip`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ClipRect {
    /// Coordenada X del borde izquierdo, en píxeles físicos.
    pub x: i32,
    /// Coordenada Y del borde superior, en píxeles físicos.
    pub y: i32,
    /// Ancho del rectángulo, en píxeles físicos.
    pub width: u32,
    /// Alto del rectángulo, en píxeles físicos.
    pub height: u32,
}

impl ClipRect {
    /// Rectángulo vacío en el origen (`0, 0, 0, 0`).
    pub const ZERO: Self = Self {
        x: 0,
        y: 0,
        width: 0,
        height: 0,
    };

    /// Crea un [`ClipRect`] en `(x, y)` con dimensiones `width` × `height`.
    #[must_use]
    pub const fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Crea un [`ClipRect`] ubicado en `(0, 0)` con el tamaño `size`.
    #[must_use]
    pub const fn from_size(size: Size) -> Self {
        Self {
            x: 0,
            y: 0,
            width: size.width,
            height: size.height,
        }
    }

    /// Borde derecho (`x + width`) con aritmética saturada.
    #[must_use]
    pub const fn right(self) -> i32 {
        self.x.saturating_add_unsigned(self.width)
    }

    /// Borde inferior (`y + height`) con aritmética saturada.
    #[must_use]
    pub const fn bottom(self) -> i32 {
        self.y.saturating_add_unsigned(self.height)
    }

    /// Comprueba si `inner` está íntegramente contenido dentro de `self`.
    #[must_use]
    pub const fn contains_rect(self, inner: Self) -> bool {
        inner.x >= self.x
            && inner.y >= self.y
            && inner.right() <= self.right()
            && inner.bottom() <= self.bottom()
    }

    /// Calcula la intersección entre `self` y `other` con aritmética saturada.
    ///
    /// Si no se solapan o alguno tiene área nula, devuelve [`ClipRect::ZERO`].
    #[must_use]
    pub const fn intersect(self, other: Self) -> Self {
        if self.width == 0 || self.height == 0 || other.width == 0 || other.height == 0 {
            return Self::ZERO;
        }
        let x1 = if self.x > other.x { self.x } else { other.x };
        let y1 = if self.y > other.y { self.y } else { other.y };
        let r1 = self.right();
        let r2 = other.right();
        let x2 = if r1 < r2 { r1 } else { r2 };
        let b1 = self.bottom();
        let b2 = other.bottom();
        let y2 = if b1 < b2 { b1 } else { b2 };

        if x2 <= x1 || y2 <= y1 {
            return Self::ZERO;
        }
        Self {
            x: x1,
            y: y1,
            width: x2.abs_diff(x1),
            height: y2.abs_diff(y1),
        }
    }
}

/// Comando de trazo vectorial plano en una grilla entera fija ([`ICON_GRID_SIZE`] = `24 × 24`).
///
/// Es una estructura de datos pura sin tipos de `tiny-skia`, de modo que `atoll-ui`
/// pueda declarar íconos como tajadas estáticas `&'static [PathCmd]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PathCmd {
    /// Mueve el cursor a `(x, y)` sin trazar.
    MoveTo {
        /// Coordenada X en la grilla.
        x: i16,
        /// Coordenada Y en la grilla.
        y: i16,
    },
    /// Traza un segmento recto hasta `(x, y)`.
    LineTo {
        /// Coordenada X final en la grilla.
        x: i16,
        /// Coordenada Y final en la grilla.
        y: i16,
    },
    /// Traza una curva cuadrática de Bézier con control `(cx, cy)` hasta `(x, y)`.
    QuadTo {
        /// Coordenada X del punto de control.
        cx: i16,
        /// Coordenada Y del punto de control.
        cy: i16,
        /// Coordenada X final.
        x: i16,
        /// Coordenada Y final.
        y: i16,
    },
    /// Traza una curva cúbica de Bézier con controles `(c1x, c1y)` y `(c2x, c2y)` hasta `(x, y)`.
    CubicTo {
        /// Coordenada X del primer control.
        c1x: i16,
        /// Coordenada Y del primer control.
        c1y: i16,
        /// Coordenada X del segundo control.
        c2x: i16,
        /// Coordenada Y del segundo control.
        c2y: i16,
        /// Coordenada X final.
        x: i16,
        /// Coordenada Y final.
        y: i16,
    },
    /// Cierra el sub-camino actual.
    Close,
}

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
    /// Crea un [`Frame`] transparente de tamaño `size`.
    ///
    /// # Errors
    ///
    /// Devuelve [`RenderError::InvalidSize`] si el ancho o el alto son cero o exceden el límite.
    pub fn new(size: Size) -> Result<Self, RenderError> {
        let pixmap = Pixmap::new(size.width, size.height).ok_or(RenderError::InvalidSize(size))?;
        Ok(Self {
            size,
            rgba: pixmap.take(),
        })
    }

    /// Crea un [`Frame`] de tamaño `size` íntegramente relleno con `color` (premultiplicado).
    ///
    /// # Errors
    ///
    /// Devuelve [`RenderError::InvalidSize`] si el ancho o el alto son cero o exceden el límite.
    pub fn new_filled(size: Size, color: Color) -> Result<Self, RenderError> {
        let mut frame = Self::new(size)?;
        fill_rounded_rect(
            &mut frame,
            ClipRect::from_size(size),
            CornerRadii::ZERO,
            color,
            ClipRect::from_size(size),
        );
        Ok(frame)
    }

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

    /// Acceso mutable a los bytes RGBA premultiplicados del [`Frame`].
    #[must_use]
    pub fn rgba_premultiplied_mut(&mut self) -> &mut [u8] {
        &mut self.rgba
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

    /// Dibuja un rectángulo con esquinas redondeadas independientes (`radii`) y color `color`
    /// dentro de `rect`, recortado estrictamente a la intersección entre `clip` y este [`Frame`].
    pub fn fill_rounded_rect(
        &mut self,
        rect: ClipRect,
        radii: CornerRadii,
        color: Color,
        clip: ClipRect,
    ) {
        fill_rounded_rect(self, rect, radii, color, clip);
    }

    /// Dibuja una secuencia de comandos de trazo (`commands`), definidos sobre una grilla
    /// lógica de [`ICON_GRID_SIZE`] × [`ICON_GRID_SIZE`] (`24×24`), escalados para caber
    /// dentro de `rect` y recortados estrictamente a `clip`.
    pub fn stroke_path(
        &mut self,
        rect: ClipRect,
        commands: &[PathCmd],
        stroke_width: f32,
        color: Color,
        scale: f32,
        clip: ClipRect,
    ) {
        stroke_path(self, commands, rect, stroke_width, scale, color, clip);
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

/// Dibuja un rectángulo con esquinas redondeadas (`radii`) relleno con `color`
/// sobre `frame`, recortando estrictamente el resultado a `clip`.
///
/// Usa aritmética saturante: si `rect` o `clip` tienen tamaño cero, caen fuera del
/// `frame` o `color.a == 0`, no modifica ningún píxel y jamás entra en pánico.
pub fn fill_rounded_rect(
    frame: &mut Frame,
    rect: ClipRect,
    radii: CornerRadii,
    color: Color,
    clip: ClipRect,
) {
    if rect.width == 0 || rect.height == 0 || color.a == 0 {
        return;
    }
    let frame_bounds = ClipRect::from_size(frame.size);
    let eff_clip = clip.intersect(frame_bounds);
    let work = eff_clip.intersect(rect);
    if work.width == 0 || work.height == 0 {
        return;
    }

    let Some(path) = rounded_rect(rect.width as f32, rect.height as f32, radii) else {
        return;
    };

    let mut paint = Paint::default();
    paint.set_color_rgba8(color.r, color.g, color.b, color.a);
    paint.anti_alias = true;

    // Caso rápido sin alojamientos: `rect` entra entero dentro del `clip` efectivo.
    if eff_clip.contains_rect(rect) {
        if let Some(mut pixmap) =
            PixmapMut::from_bytes(&mut frame.rgba, frame.size.width, frame.size.height)
        {
            pixmap.fill_path(
                &path,
                &paint,
                FillRule::Winding,
                Transform::from_translate(rect.x as f32, rect.y as f32),
                None,
            );
        }
        return;
    }

    // Caso recortado: dibujamos sobre una sub-superficie del tamaño exacto de `work`
    // para garantizar que ni un solo píxel fuera de `eff_clip` se modifique.
    let Some(mut sub) = copy_sub_pixmap(frame, work) else {
        return;
    };
    let tx = (i64::from(rect.x) - i64::from(work.x)) as f32;
    let ty = (i64::from(rect.y) - i64::from(work.y)) as f32;
    sub.as_mut().fill_path(
        &path,
        &paint,
        FillRule::Winding,
        Transform::from_translate(tx, ty),
        None,
    );
    write_sub_pixmap(frame, work, &sub);
}

/// Traza una lista de comandos vectoriales [`PathCmd`] (definidos en una grilla de
/// [`ICON_GRID_SIZE`] × [`ICON_GRID_SIZE`]) escalados al rectángulo `rect`, con grosor
/// lógico `stroke_width` multiplicado por `scale`, color `color` y recorte estricto a `clip`.
///
/// Usa aritmética saturante: si `rect` o `clip` son degenerados, están fuera del `frame`
/// o los parámetros no son positivos/finitos, no hace nada y jamás entra en pánico.
pub fn stroke_path(
    frame: &mut Frame,
    commands: &[PathCmd],
    rect: ClipRect,
    stroke_width: f32,
    scale: f32,
    color: Color,
    clip: ClipRect,
) {
    if commands.is_empty()
        || rect.width == 0
        || rect.height == 0
        || color.a == 0
        || !stroke_width.is_finite()
        || stroke_width <= 0.0
        || !scale.is_finite()
        || scale <= 0.0
    {
        return;
    }

    let frame_bounds = ClipRect::from_size(frame.size);
    let work = clip.intersect(rect).intersect(frame_bounds);
    if work.width == 0 || work.height == 0 {
        return;
    }

    let grid = f32::from(ICON_GRID_SIZE);
    let sx = (rect.width as f32) / grid;
    let sy = (rect.height as f32) / grid;
    let map_x = |gx: i16| -> f32 { f32::from(gx) * sx };
    let map_y = |gy: i16| -> f32 { f32::from(gy) * sy };

    let mut builder = PathBuilder::new();
    for &cmd in commands {
        match cmd {
            PathCmd::MoveTo { x, y } => builder.move_to(map_x(x), map_y(y)),
            PathCmd::LineTo { x, y } => builder.line_to(map_x(x), map_y(y)),
            PathCmd::QuadTo { cx, cy, x, y } => {
                builder.quad_to(map_x(cx), map_y(cy), map_x(x), map_y(y));
            }
            PathCmd::CubicTo {
                c1x,
                c1y,
                c2x,
                c2y,
                x,
                y,
            } => {
                builder.cubic_to(
                    map_x(c1x),
                    map_y(c1y),
                    map_x(c2x),
                    map_y(c2y),
                    map_x(x),
                    map_y(y),
                );
            }
            PathCmd::Close => builder.close(),
        }
    }

    let Some(path) = builder.finish() else {
        return;
    };

    let min_dim = (rect.width.min(rect.height) as f32).max(1.0);
    let phys_width = (stroke_width * scale).clamp(1.0, (min_dim / 2.0).max(1.0));
    let stroke = Stroke {
        width: phys_width,
        line_cap: LineCap::Round,
        line_join: LineJoin::Round,
        ..Stroke::default()
    };

    let mut paint = Paint::default();
    paint.set_color_rgba8(color.r, color.g, color.b, color.a);
    paint.anti_alias = true;

    let Some(mut sub) = copy_sub_pixmap(frame, work) else {
        return;
    };
    let tx = (i64::from(rect.x) - i64::from(work.x)) as f32;
    let ty = (i64::from(rect.y) - i64::from(work.y)) as f32;
    sub.as_mut().stroke_path(
        &path,
        &paint,
        &stroke,
        Transform::from_translate(tx, ty),
        None,
    );
    write_sub_pixmap(frame, work, &sub);
}

/// Copia la región `work` (que ya debe estar dentro de `frame`) a un [`Pixmap`] temporal.
fn copy_sub_pixmap(frame: &Frame, work: ClipRect) -> Option<Pixmap> {
    let mut sub = Pixmap::new(work.width, work.height)?;
    let frame_w = frame.size.width as usize;
    let work_w = work.width as usize;
    let work_h = work.height as usize;
    let start_x = usize::try_from(work.x).ok()?;
    let start_y = usize::try_from(work.y).ok()?;

    let dst_bytes = sub.data_mut();
    for row in 0..work_h {
        let src_row_start = (start_y + row)
            .checked_mul(frame_w)?
            .checked_add(start_x)?
            .checked_mul(4)?;
        let src_row_end = src_row_start.checked_add(work_w.checked_mul(4)?)?;
        let dst_row_start = row.checked_mul(work_w)?.checked_mul(4)?;
        let dst_row_end = dst_row_start.checked_add(work_w.checked_mul(4)?)?;
        let src_slice = frame.rgba.get(src_row_start..src_row_end)?;
        let dst_slice = dst_bytes.get_mut(dst_row_start..dst_row_end)?;
        dst_slice.copy_from_slice(src_slice);
    }
    Some(sub)
}

/// Escribe de vuelta los píxeles de `sub` en la región `work` de `frame`.
fn write_sub_pixmap(frame: &mut Frame, work: ClipRect, sub: &Pixmap) {
    let frame_w = frame.size.width as usize;
    let work_w = work.width as usize;
    let work_h = work.height as usize;
    let Ok(start_x) = usize::try_from(work.x) else {
        return;
    };
    let Ok(start_y) = usize::try_from(work.y) else {
        return;
    };

    let src_bytes = sub.data();
    for row in 0..work_h {
        let Some(dst_row_start) = (start_y + row)
            .checked_mul(frame_w)
            .and_then(|v| v.checked_add(start_x))
            .and_then(|v| v.checked_mul(4))
        else {
            return;
        };
        let Some(dst_row_end) = dst_row_start.checked_add(work_w.saturating_mul(4)) else {
            return;
        };
        let Some(src_row_start) = row.checked_mul(work_w).and_then(|v| v.checked_mul(4)) else {
            return;
        };
        let Some(src_row_end) = src_row_start.checked_add(work_w.saturating_mul(4)) else {
            return;
        };
        if let (Some(src_slice), Some(dst_slice)) = (
            src_bytes.get(src_row_start..src_row_end),
            frame.rgba.get_mut(dst_row_start..dst_row_end),
        ) {
            dst_slice.copy_from_slice(src_slice);
        }
    }
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
