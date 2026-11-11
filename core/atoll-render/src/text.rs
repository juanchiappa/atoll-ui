//! Primitiva de texto: rasteriza glifos de una fuente embebida y los compone sobre un [`Frame`].
//!
//! Diseño:
//! - La fuente (Inter Medium recortada, ver `THIRD_PARTY_NOTICES.md`) viaja dentro del
//!   binario con `include_bytes!`. No se leen fuentes del sistema.
//! - Cada glifo se rasteriza una sola vez por tamaño y queda en una caché acotada: redibujar
//!   el mismo texto no vuelve a rasterizar nada.
//! - No hay estado global ni hilos: quien dibuja es dueño de un [`TextRenderer`].
//! - Se dibuja solo cuando alguien lo pide; no existe ningún loop.

use std::collections::HashMap;

use fontdue::{Font, FontSettings};

use crate::{Frame, RenderError};

/// Fuente embebida: Inter Medium recortada a dígitos, signos básicos y letras del español.
const FONT_BYTES: &[u8] = include_bytes!("../assets/Inter-Medium-atoll.ttf");

/// Tamaño máximo de fuente aceptado, en píxeles.
const MAX_FONT_PX: f32 = 512.0;

/// Límite de coordenadas aceptadas, en píxeles. Evita desbordes al sumar posiciones.
const MAX_COORD: f32 = 1_000_000.0;

/// Cantidad máxima de glifos en caché. Al superarla se vacía la caché entera.
const MAX_CACHED_GLYPHS: usize = 512;

/// Color RGBA de 8 bits, sin premultiplicar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    /// Rojo.
    pub r: u8,
    /// Verde.
    pub g: u8,
    /// Azul.
    pub b: u8,
    /// Opacidad (255 es opaco).
    pub a: u8,
}

impl Color {
    /// Negro opaco.
    pub const BLACK: Self = Self::rgba(0, 0, 0, 255);

    /// Blanco opaco.
    pub const WHITE: Self = Self::rgba(255, 255, 255, 255);

    /// Crea un color a partir de sus cuatro canales.
    #[must_use]
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
}

/// Medidas verticales de una línea de texto, en píxeles.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineMetrics {
    /// Distancia de la línea base hacia arriba (positiva).
    pub ascent: f32,
    /// Distancia de la línea base hacia abajo (negativa).
    pub descent: f32,
}

/// Clave de caché: carácter y tamaño (bits del `f32`, que siempre es finito).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct GlyphKey {
    ch: char,
    size_bits: u32,
}

/// Glifo ya rasterizado: cobertura de 8 bits y su posición respecto del origen.
#[derive(Debug, Clone)]
struct Glyph {
    width: usize,
    height: usize,
    /// Desplazamiento horizontal del borde izquierdo respecto del origen.
    xmin: i32,
    /// Desplazamiento vertical del borde inferior respecto de la línea base (positivo = arriba).
    ymin: i32,
    /// Avance horizontal hasta el próximo glifo.
    advance: f32,
    coverage: Vec<u8>,
}

/// Rasteriza texto con la fuente embebida y guarda los glifos en caché.
#[derive(Debug)]
pub struct TextRenderer {
    font: Font,
    cache: HashMap<GlyphKey, Glyph>,
}

impl TextRenderer {
    /// Carga la fuente embebida.
    ///
    /// # Errors
    ///
    /// Devuelve [`RenderError::InvalidFont`] si la fuente embebida no se puede interpretar.
    pub fn new() -> Result<Self, RenderError> {
        let settings = FontSettings {
            load_substitutions: false,
            ..FontSettings::default()
        };
        let font = Font::from_bytes(FONT_BYTES, settings).map_err(|_| RenderError::InvalidFont)?;
        Ok(Self {
            font,
            cache: HashMap::new(),
        })
    }

    /// Cantidad de glifos que hay hoy en caché.
    #[must_use]
    pub fn cached_glyphs(&self) -> usize {
        self.cache.len()
    }

    /// Vacía la caché de glifos.
    ///
    /// Se usa cuando cambia el factor de escala (DPI): los glifos rasterizados para
    /// el tamaño anterior ya no se van a pedir, y no tiene sentido guardarlos.
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Medidas verticales de la fuente al tamaño pedido.
    ///
    /// # Errors
    ///
    /// Devuelve [`RenderError::InvalidTextParams`] si `size_px` no es válido, o
    /// [`RenderError::InvalidFont`] si la fuente no trae medidas de línea.
    pub fn line_metrics(&self, size_px: f32) -> Result<LineMetrics, RenderError> {
        check_size(size_px)?;
        self.font
            .horizontal_line_metrics(size_px)
            .map(|m| LineMetrics {
                ascent: m.ascent,
                descent: m.descent,
            })
            .ok_or(RenderError::InvalidFont)
    }

    /// Ancho en píxeles que ocupa `text` al tamaño pedido.
    ///
    /// # Errors
    ///
    /// Devuelve [`RenderError::InvalidTextParams`] si `size_px` no es válido.
    pub fn measure(&mut self, text: &str, size_px: f32) -> Result<f32, RenderError> {
        check_size(size_px)?;
        let mut width = 0.0;
        for ch in text.chars().filter(|c| !c.is_control()) {
            width += self.glyph(ch, size_px).advance;
        }
        Ok(width)
    }

    /// Dibuja `text` sobre `frame` y devuelve el ancho que ocupó.
    ///
    /// `x` es el borde izquierdo del texto e `y` la línea base, ambos en píxeles del frame.
    /// Lo que cae fuera del frame se recorta. El color se compone sobre lo que ya hay
    /// (alfa premultiplicado, igual que el frame). Los caracteres que la fuente no trae se
    /// dibujan con el glifo de "carácter faltante"; los de control se ignoran.
    ///
    /// # Errors
    ///
    /// Devuelve [`RenderError::InvalidTextParams`] si `size_px` no es válido o si `x` o `y`
    /// no son finitos o están fuera de rango.
    pub fn draw(
        &mut self,
        frame: &mut Frame,
        text: &str,
        x: f32,
        y: f32,
        size_px: f32,
        color: Color,
    ) -> Result<f32, RenderError> {
        check_size(size_px)?;
        check_coord(x)?;
        check_coord(y)?;

        let baseline = y.round() as i64;
        let mut pen = x;
        for ch in text.chars().filter(|c| !c.is_control()) {
            let glyph = self.glyph(ch, size_px);
            let left = pen.round() as i64 + i64::from(glyph.xmin);
            let top = baseline - i64::from(glyph.ymin) - glyph.height as i64;
            blit(frame, glyph, left, top, color);
            pen += glyph.advance;
        }
        Ok(pen - x)
    }

    /// Devuelve el glifo rasterizado, de la caché si ya estaba.
    fn glyph(&mut self, ch: char, size_px: f32) -> &Glyph {
        let key = GlyphKey {
            ch,
            size_bits: size_px.to_bits(),
        };
        if self.cache.len() >= MAX_CACHED_GLYPHS && !self.cache.contains_key(&key) {
            self.cache.clear();
        }
        let font = &self.font;
        self.cache.entry(key).or_insert_with(|| {
            let (metrics, coverage) = font.rasterize(ch, size_px);
            Glyph {
                width: metrics.width,
                height: metrics.height,
                xmin: metrics.xmin,
                ymin: metrics.ymin,
                advance: metrics.advance_width,
                coverage,
            }
        })
    }
}

/// Valida que el tamaño de fuente sea finito, positivo y razonable.
fn check_size(size_px: f32) -> Result<(), RenderError> {
    if size_px.is_finite() && size_px > 0.0 && size_px <= MAX_FONT_PX {
        Ok(())
    } else {
        Err(RenderError::InvalidTextParams)
    }
}

/// Valida que una coordenada sea finita y no se pase del rango aceptado.
fn check_coord(value: f32) -> Result<(), RenderError> {
    if value.is_finite() && value.abs() <= MAX_COORD {
        Ok(())
    } else {
        Err(RenderError::InvalidTextParams)
    }
}

/// Divide por 255 con redondeo, para `a * b` con `a` y `b` en `0..=255`.
const fn mul_div_255(a: u32, b: u32) -> u32 {
    let t = a * b + 128;
    (t + (t >> 8)) >> 8
}

/// Compone la cobertura de un glifo con `color` sobre el frame, recortando lo que sobra.
fn blit(frame: &mut Frame, glyph: &Glyph, left: i64, top: i64, color: Color) {
    let frame_w = i64::from(frame.size.width);
    let frame_h = i64::from(frame.size.height);
    if glyph.width == 0 || glyph.height == 0 {
        return;
    }

    for row in 0..glyph.height {
        let y = top + row as i64;
        if y < 0 || y >= frame_h {
            continue;
        }
        for col in 0..glyph.width {
            let x = left + col as i64;
            if x < 0 || x >= frame_w {
                continue;
            }
            let Some(&coverage) = glyph.coverage.get(row * glyph.width + col) else {
                continue;
            };
            if coverage == 0 {
                continue;
            }
            let Ok(start) = usize::try_from((y * frame_w + x) * 4) else {
                continue;
            };
            let Some(pixel) = frame.rgba.get_mut(start..start + 4) else {
                continue;
            };
            let [dr, dg, db, da] = pixel else {
                continue;
            };

            let alpha = mul_div_255(u32::from(coverage), u32::from(color.a));
            let inverse = 255 - alpha;
            let blend = |src: u8, dst: u8| -> u8 {
                let value =
                    mul_div_255(u32::from(src), alpha) + mul_div_255(u32::from(dst), inverse);
                u8::try_from(value.min(255)).unwrap_or(255)
            };
            *dr = blend(color.r, *dr);
            *dg = blend(color.g, *dg);
            *db = blend(color.b, *db);
            *da = blend(255, *da);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Color, TextRenderer, MAX_CACHED_GLYPHS};
    use crate::{render_island, Frame, RenderError, Size};

    /// Frame totalmente transparente.
    fn transparent(width: u32, height: u32) -> Frame {
        Frame {
            size: Size::new(width, height),
            rgba: vec![0; (width * height * 4) as usize],
        }
    }

    /// Píxel RGBA (premultiplicado) en (x, y).
    fn pixel(frame: &Frame, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * frame.size().width + x) * 4) as usize;
        let px = &frame.rgba_premultiplied()[i..i + 4];
        [px[0], px[1], px[2], px[3]]
    }

    /// Caja delimitadora (x_min, y_min, x_max, y_max) de los píxeles con alfa > 0.
    fn bounds(frame: &Frame) -> Option<(u32, u32, u32, u32)> {
        let mut found: Option<(u32, u32, u32, u32)> = None;
        for y in 0..frame.size().height {
            for x in 0..frame.size().width {
                if pixel(frame, x, y)[3] > 0 {
                    found = Some(match found {
                        None => (x, y, x, y),
                        Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
                    });
                }
            }
        }
        found
    }

    fn renderer() -> TextRenderer {
        TextRenderer::new().expect("la fuente embebida carga")
    }

    #[test]
    fn embedded_font_has_every_character_we_need() {
        let r = renderer();
        let needed =
            "0123456789:./,- abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZáéíóúñÁÉÍÓÚÑ";
        for ch in needed.chars().filter(|c| *c != ' ') {
            assert_ne!(
                r.font.lookup_glyph_index(ch),
                0,
                "falta {ch:?} en la fuente"
            );
        }
    }

    #[test]
    fn draw_leaves_pixels_only_where_the_text_is() {
        let mut r = renderer();
        let mut frame = transparent(120, 40);
        let width = r
            .draw(&mut frame, "12:34", 10.0, 28.0, 20.0, Color::WHITE)
            .expect("dibuja");
        let (x0, y0, x1, y1) = bounds(&frame).expect("hay píxeles dibujados");

        assert!(x0 >= 10, "nada a la izquierda del origen, x0={x0}");
        assert!(y1 <= 30, "nada bajo la línea base más el descenso, y1={y1}");
        assert!(y0 >= 8, "los dígitos tienen altura de unos 14 px, y0={y0}");
        assert!(
            (x1 as f32) <= 10.0 + width + 1.0,
            "el texto no pasa del ancho medido"
        );
        assert!(width > 40.0 && width < 80.0, "ancho razonable, fue {width}");
    }

    #[test]
    fn white_text_is_premultiplied_white() {
        let mut r = renderer();
        let mut frame = transparent(60, 30);
        r.draw(&mut frame, "8", 5.0, 22.0, 20.0, Color::WHITE)
            .expect("dibuja");
        let full = (0..30)
            .flat_map(|y| (0..60).map(move |x| (x, y)))
            .map(|(x, y)| pixel(&frame, x, y))
            .find(|px| px[3] == 255)
            .expect("el trazo tiene píxeles de cobertura total");
        assert_eq!(full, [255, 255, 255, 255]);
        for y in 0..30 {
            for x in 0..60 {
                let [rr, g, b, a] = pixel(&frame, x, y);
                assert!(
                    rr == a && g == a && b == a,
                    "premultiplicado: {rr},{g},{b},{a}"
                );
            }
        }
    }

    #[test]
    fn colored_text_keeps_its_color() {
        let mut r = renderer();
        let mut frame = transparent(60, 30);
        r.draw(
            &mut frame,
            "I",
            5.0,
            22.0,
            24.0,
            Color::rgba(255, 0, 0, 255),
        )
        .expect("dibuja");
        let (x0, y0, x1, y1) = bounds(&frame).expect("hay píxeles");
        for y in y0..=y1 {
            for x in x0..=x1 {
                let [rr, g, b, a] = pixel(&frame, x, y);
                assert_eq!((g, b), (0, 0));
                assert_eq!(rr, a);
            }
        }
    }

    #[test]
    fn text_over_the_island_keeps_it_opaque_and_adds_light() {
        let mut r = renderer();
        let mut frame = render_island(Size::new(400, 40)).expect("isla");
        r.draw(&mut frame, "12:34", 150.0, 27.0, 20.0, Color::WHITE)
            .expect("dibuja");

        assert_eq!(pixel(&frame, 0, 0)[3], 0, "la esquina sigue transparente");
        assert_eq!(pixel(&frame, 200, 20)[3], 255, "el centro sigue opaco");
        let brightest = (150..230)
            .flat_map(|x| (5..35).map(move |y| (x, y)))
            .map(|(x, y)| pixel(&frame, x, y)[0])
            .max()
            .expect("hay píxeles");
        assert_eq!(brightest, 255, "hay trazo blanco pleno sobre la isla");
    }

    #[test]
    fn redrawing_does_not_rasterize_again() {
        let mut r = renderer();
        let mut first = transparent(120, 40);
        r.draw(&mut first, "12:34", 10.0, 28.0, 20.0, Color::WHITE)
            .expect("dibuja");
        assert_eq!(r.cached_glyphs(), 5, "1, 2, :, 3 y 4");

        let mut second = transparent(120, 40);
        r.draw(&mut second, "12:34", 10.0, 28.0, 20.0, Color::WHITE)
            .expect("dibuja");
        assert_eq!(r.cached_glyphs(), 5, "no hay glifos nuevos");
        assert_eq!(first, second, "mismo texto, mismos píxeles");

        r.draw(&mut second, "12:35", 10.0, 28.0, 20.0, Color::WHITE)
            .expect("dibuja");
        assert_eq!(r.cached_glyphs(), 6, "solo se suma el 5");
    }

    #[test]
    fn clearing_the_cache_forgets_glyphs_but_not_how_to_draw_them() {
        let mut r = renderer();
        let mut first = transparent(120, 40);
        r.draw(&mut first, "12:34", 10.0, 28.0, 20.0, Color::WHITE)
            .expect("dibuja");
        assert_eq!(r.cached_glyphs(), 5);

        r.clear_cache();
        assert_eq!(r.cached_glyphs(), 0);

        let mut again = transparent(120, 40);
        r.draw(&mut again, "12:34", 10.0, 28.0, 20.0, Color::WHITE)
            .expect("dibuja");
        assert_eq!(first, again, "se rasteriza igual que antes");
    }

    #[test]
    fn cache_is_bounded() {
        let mut r = renderer();
        let mut frame = transparent(40, 40);
        for step in 0..700_u16 {
            let size = 8.0 + f32::from(step) * 0.25;
            r.draw(&mut frame, "1", 0.0, 30.0, size, Color::WHITE)
                .expect("dibuja");
            assert!(r.cached_glyphs() <= MAX_CACHED_GLYPHS);
        }
    }

    #[test]
    fn measure_matches_draw_and_scales_with_size() {
        let mut r = renderer();
        let small = r.measure("12:34", 10.0).expect("mide");
        let big = r.measure("12:34", 20.0).expect("mide");
        assert!((big / small - 2.0).abs() < 0.1, "{small} vs {big}");
        assert!(r.measure("1", 20.0).expect("mide") < big);
        assert_eq!(r.measure("", 20.0).expect("mide"), 0.0);

        let mut frame = transparent(200, 40);
        let drawn = r
            .draw(&mut frame, "12:34", 0.0, 28.0, 20.0, Color::WHITE)
            .expect("dibuja");
        assert!((drawn - big).abs() < f32::EPSILON);
    }

    #[test]
    fn line_metrics_are_sane() {
        let r = renderer();
        let m = r.line_metrics(20.0).expect("medidas");
        assert!(m.ascent > 10.0 && m.ascent < 25.0, "{m:?}");
        assert!(m.descent < 0.0 && m.descent > -10.0, "{m:?}");
    }

    #[test]
    fn drawing_outside_the_frame_is_clipped_not_fatal() {
        let mut r = renderer();
        let mut frame = transparent(50, 20);
        let blank = frame.clone();

        for (x, y) in [(-500.0, 10.0), (500.0, 10.0), (10.0, -500.0), (10.0, 500.0)] {
            r.draw(&mut frame, "12:34", x, y, 20.0, Color::WHITE)
                .expect("recorta");
            assert_eq!(frame, blank, "fuera del frame no queda nada ({x}, {y})");
        }

        r.draw(&mut frame, "12:34", -10.0, 18.0, 20.0, Color::WHITE)
            .expect("recorta");
        assert!(
            bounds(&frame).is_some(),
            "lo que entra parcialmente se dibuja"
        );
        assert_eq!(frame.size(), Size::new(50, 20));
    }

    #[test]
    fn invalid_parameters_are_rejected() {
        let mut r = renderer();
        let mut frame = transparent(50, 20);
        for size in [0.0, -4.0, f32::NAN, f32::INFINITY, 513.0] {
            assert_eq!(
                r.draw(&mut frame, "1", 0.0, 10.0, size, Color::WHITE),
                Err(RenderError::InvalidTextParams),
                "tamaño {size}"
            );
            assert_eq!(r.measure("1", size), Err(RenderError::InvalidTextParams));
        }
        for (x, y) in [(f32::NAN, 0.0), (0.0, f32::INFINITY), (2e6, 0.0)] {
            assert_eq!(
                r.draw(&mut frame, "1", x, y, 12.0, Color::WHITE),
                Err(RenderError::InvalidTextParams)
            );
        }
    }

    #[test]
    fn unsupported_and_control_characters_do_not_break_drawing() {
        let mut r = renderer();
        let mut frame = transparent(120, 40);
        r.draw(&mut frame, "日€\n\t7", 5.0, 28.0, 20.0, Color::WHITE)
            .expect("dibuja");
        assert!(bounds(&frame).is_some());
        let with_control = r.measure("7\n", 20.0).expect("mide");
        let without = r.measure("7", 20.0).expect("mide");
        assert!((with_control - without).abs() < f32::EPSILON);
    }
}
