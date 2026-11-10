//! Contenido de la isla: la hora y, al pasar el mouse, la fecha.
//!
//! El backend de la plataforma llama a [`IslandPainter`] cada vez que hay que
//! dibujar. Acá se decide qué se ve y dónde; el backend solo lo entrega al SO.
//!
//! Las medidas de este módulo (tamaño de letra, separaciones) están en píxeles
//! lógicos, a 96 DPI. Al dibujar se multiplican por el factor de escala del monitor.

use atoll_platform::dpi::scale_px;
use atoll_platform::{LocalTime, Painter};
use atoll_render::{render_island, Color, Frame, RenderError, Size, TextRenderer};

/// Días de la semana abreviados, empezando por el domingo.
const WEEKDAYS: [&str; 7] = ["dom", "lun", "mar", "mié", "jue", "vie", "sáb"];

/// Meses abreviados, empezando por enero.
const MONTHS: [&str; 12] = [
    "ene", "feb", "mar", "abr", "may", "jun", "jul", "ago", "sep", "oct", "nov", "dic",
];

/// Texto que se muestra si el día o el mes vienen fuera de rango.
const UNKNOWN: &str = "---";

/// Tamaño de la hora, en píxeles lógicos.
const TIME_PX: f32 = 22.0;

/// Tamaño de la fecha, en píxeles lógicos.
const DATE_PX: f32 = 13.0;

/// Altura de las mayúsculas y los dígitos de Inter, como fracción del tamaño (1490 / 2048).
const CAP_HEIGHT: f32 = 0.7275;

/// Separación entre la hora y la fecha, en píxeles lógicos.
const LINE_GAP: f32 = 8.0;

/// Color de la hora.
const TIME_COLOR: Color = Color::WHITE;

/// Opacidad máxima de la fecha (0 a 255), cuando la isla está totalmente expandida.
const DATE_ALPHA: f32 = 190.0;

/// Hora en formato 24 h: `HH:MM`.
#[must_use]
pub fn format_time(time: &LocalTime) -> String {
    format!("{:02}:{:02}", time.hour, time.minute)
}

/// Fecha abreviada en español, por ejemplo `sáb 7 nov`.
#[must_use]
pub fn format_date(time: &LocalTime) -> String {
    let weekday = WEEKDAYS.get(usize::from(time.weekday)).unwrap_or(&UNKNOWN);
    let month = usize::from(time.month)
        .checked_sub(1)
        .and_then(|index| MONTHS.get(index))
        .unwrap_or(&UNKNOWN);
    format!("{weekday} {} {month}", time.day)
}

/// Dibuja la isla con la hora, y la fecha que aparece al expandirse.
#[derive(Debug)]
pub struct IslandPainter {
    text: TextRenderer,
    /// Tamaño lógico de la isla en reposo.
    collapsed: Size,
    /// Tamaño lógico de la isla expandida.
    expanded: Size,
    /// Escala con la que se dibujó el último cuadro (bits del `f32`), para saber cuándo cambia.
    last_scale: u32,
}

impl IslandPainter {
    /// Crea el painter para una isla que va de `collapsed` a `expanded` (tamaños lógicos).
    ///
    /// # Errors
    ///
    /// Devuelve [`RenderError`] si no se puede cargar la fuente embebida.
    pub fn new(collapsed: Size, expanded: Size) -> Result<Self, RenderError> {
        Ok(Self {
            text: TextRenderer::new()?,
            collapsed,
            expanded,
            last_scale: 1.0_f32.to_bits(),
        })
    }

    /// Qué tan expandida está la isla: 0,0 en reposo y 1,0 con hover.
    ///
    /// `size` es el tamaño físico del cuadro; los tamaños de reposo y expandido se
    /// escalan con `scale` para compararlos en las mismas unidades.
    fn progress(&self, size: Size, scale: f32) -> f32 {
        let collapsed = scale_px(self.collapsed.height, scale);
        let expanded = scale_px(self.expanded.height, scale);
        let span = expanded.saturating_sub(collapsed);
        if span == 0 {
            return 0.0;
        }
        let done = size.height.saturating_sub(collapsed);
        (done as f32 / span as f32).clamp(0.0, 1.0)
    }

    /// Dibuja un cuadro completo: isla, hora y fecha, a la escala `scale`.
    fn draw(&mut self, size: Size, scale: f32, time: &LocalTime) -> Result<Frame, RenderError> {
        // Una escala inválida (no finita o no positiva) se trata como 100 %.
        let scale = if scale.is_finite() && scale > 0.0 {
            scale
        } else {
            1.0
        };
        // Al cambiar la escala, los glifos cacheados del tamaño anterior ya no sirven.
        if scale.to_bits() != self.last_scale {
            self.text.clear_cache();
            self.last_scale = scale.to_bits();
        }

        let mut frame = render_island(size)?;
        let progress = self.progress(size, scale);
        let width = size.width as f32;
        let height = size.height as f32;

        let time_px = TIME_PX * scale;
        let date_px = DATE_PX * scale;
        let line_gap = LINE_GAP * scale;
        let time_cap = time_px * CAP_HEIGHT;
        let date_cap = date_px * CAP_HEIGHT;

        // En reposo la hora va centrada; expandida, hora y fecha forman un bloque centrado.
        // La línea base de la hora se mueve de una posición a la otra junto con la isla.
        let rest_baseline = (height + time_cap) / 2.0;
        let block = time_cap + line_gap + date_cap;
        let expanded_baseline = (height - block) / 2.0 + time_cap;
        let time_baseline = rest_baseline + (expanded_baseline - rest_baseline) * progress;

        let time_text = format_time(time);
        let time_width = self.text.measure(&time_text, time_px)?;
        self.text.draw(
            &mut frame,
            &time_text,
            (width - time_width) / 2.0,
            time_baseline,
            time_px,
            TIME_COLOR,
        )?;

        // La fecha aparece con la expansión: transparente en reposo, visible con hover.
        let alpha = (progress * DATE_ALPHA).round();
        if alpha >= 1.0 {
            let date_text = format_date(time);
            let date_width = self.text.measure(&date_text, date_px)?;
            let color = Color::rgba(255, 255, 255, alpha as u8);
            self.text.draw(
                &mut frame,
                &date_text,
                (width - date_width) / 2.0,
                time_baseline + line_gap + date_cap,
                date_px,
                color,
            )?;
        }
        Ok(frame)
    }
}

impl Painter for IslandPainter {
    fn paint(&mut self, size: Size, scale: f32, time: LocalTime) -> Option<Frame> {
        self.draw(size, scale, &time).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::{format_date, format_time, IslandPainter};
    use atoll_platform::{LocalTime, Painter};
    use atoll_render::{Frame, Size};

    const REST: Size = Size::new(400, 40);
    const HOVER: Size = Size::new(480, 64);

    /// Sábado 7 de noviembre de 2026, 09:05.
    fn saturday() -> LocalTime {
        LocalTime {
            year: 2026,
            month: 11,
            day: 7,
            weekday: 6,
            hour: 9,
            minute: 5,
            second: 30,
            millisecond: 0,
        }
    }

    fn painter() -> IslandPainter {
        IslandPainter::new(REST, HOVER).expect("la fuente embebida carga")
    }

    fn paint(painter: &mut IslandPainter, size: Size, time: LocalTime) -> Frame {
        painter.paint(size, 1.0, time).expect("dibuja")
    }

    fn paint_scaled(painter: &mut IslandPainter, size: Size, scale: f32) -> Frame {
        painter.paint(size, scale, saturday()).expect("dibuja")
    }

    /// Píxel RGBA (premultiplicado) en (x, y).
    fn pixel(frame: &Frame, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * frame.size().width + x) * 4) as usize;
        let px = &frame.rgba_premultiplied()[i..i + 4];
        [px[0], px[1], px[2], px[3]]
    }

    /// Caja delimitadora de los píxeles con luz (rojo > 0) dentro de las filas `rows`.
    fn light_bounds(frame: &Frame, rows: std::ops::Range<u32>) -> Option<(u32, u32, u32, u32)> {
        let mut found: Option<(u32, u32, u32, u32)> = None;
        for y in rows {
            for x in 0..frame.size().width {
                if pixel(frame, x, y)[0] > 0 {
                    found = Some(match found {
                        None => (x, y, x, y),
                        Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
                    });
                }
            }
        }
        found
    }

    fn brightest(frame: &Frame, rows: std::ops::Range<u32>) -> u8 {
        rows.flat_map(|y| (0..frame.size().width).map(move |x| (x, y)))
            .map(|(x, y)| pixel(frame, x, y)[0])
            .max()
            .unwrap_or(0)
    }

    #[test]
    fn time_is_24_hour_with_leading_zeros() {
        assert_eq!(format_time(&saturday()), "09:05");
        let mut late = saturday();
        late.hour = 23;
        late.minute = 59;
        assert_eq!(format_time(&late), "23:59");
    }

    #[test]
    fn date_is_short_spanish() {
        assert_eq!(format_date(&saturday()), "sáb 7 nov");
        let mut sunday = saturday();
        sunday.weekday = 0;
        sunday.month = 1;
        sunday.day = 31;
        assert_eq!(format_date(&sunday), "dom 31 ene");
    }

    #[test]
    fn out_of_range_date_parts_do_not_panic() {
        let mut broken = saturday();
        broken.weekday = 9;
        broken.month = 0;
        assert_eq!(format_date(&broken), "--- 7 ---");
        broken.month = 13;
        assert_eq!(format_date(&broken), "--- 7 ---");
    }

    #[test]
    fn at_rest_only_the_time_is_shown_and_centered() {
        let mut p = painter();
        let frame = paint(&mut p, REST, saturday());
        assert_eq!(frame.size(), REST);

        let (x0, y0, x1, y1) = light_bounds(&frame, 0..40).expect("se ve la hora");
        let center_x = f64::from(x0 + x1) / 2.0;
        let center_y = f64::from(y0 + y1) / 2.0;
        assert!((center_x - 200.0).abs() <= 3.0, "centrada en x: {center_x}");
        assert!((center_y - 20.0).abs() <= 2.0, "centrada en y: {center_y}");
        assert!(x0 > 150 && x1 < 250, "la hora ocupa poco ancho: {x0}..{x1}");
        assert_eq!(pixel(&frame, 0, 0)[3], 0, "la esquina sigue transparente");
    }

    #[test]
    fn hovering_shows_time_and_date_as_two_lines() {
        let mut p = painter();
        let frame = paint(&mut p, HOVER, saturday());
        assert_eq!(frame.size(), HOVER);

        let upper = light_bounds(&frame, 0..32).expect("hora arriba");
        let lower = light_bounds(&frame, 34..64).expect("fecha abajo");
        assert!(upper.3 < lower.1, "la hora queda arriba de la fecha");
        assert!(lower.2 - lower.0 > 40, "la fecha tiene ancho de texto");

        let block_top = f64::from(upper.1);
        let block_bottom = f64::from(lower.3);
        let margin_top = block_top;
        let margin_bottom = 64.0 - block_bottom;
        assert!(
            (margin_top - margin_bottom).abs() <= 5.0,
            "bloque centrado: {margin_top} arriba, {margin_bottom} abajo"
        );
    }

    #[test]
    fn date_fades_in_while_the_island_grows() {
        let mut p = painter();
        let rest = paint(&mut p, REST, saturday());
        let middle = paint(&mut p, Size::new(440, 52), saturday());
        let full = paint(&mut p, HOVER, saturday());

        assert!(light_bounds(&rest, 30..40).is_none(), "sin fecha en reposo");
        let faint = brightest(&middle, 36..52);
        let strong = brightest(&full, 36..64);
        assert!(faint > 0 && faint < strong, "{faint} < {strong}");
    }

    #[test]
    fn a_new_minute_changes_the_picture_and_the_same_minute_does_not() {
        let mut p = painter();
        let first = paint(&mut p, REST, saturday());
        let again = paint(&mut p, REST, saturday());
        assert_eq!(first, again);

        let mut later = saturday();
        later.minute = 6;
        let next = paint(&mut p, REST, later);
        assert_ne!(first, next);
    }

    #[test]
    fn text_stays_inside_the_island_at_every_size_of_the_animation() {
        let mut p = painter();
        for height in 40..=64 {
            let size = Size::new(400 + (height - 40) * 80 / 24, height);
            let frame = paint(&mut p, size, saturday());
            let (x0, _, x1, _) = light_bounds(&frame, 0..height).expect("hay texto");
            assert!(x0 > 40 && x1 < size.width - 40, "{size:?}: {x0}..{x1}");
        }
    }

    #[test]
    fn at_rest_the_text_grows_with_the_scale_and_stays_centered() {
        let mut p = painter();
        let base = paint(&mut p, REST, saturday());
        let base_box = light_bounds(&base, 0..40).expect("hora a 100 %");
        let base_height = f64::from(base_box.3 - base_box.1);

        for (scale, size) in [
            (1.25, Size::new(500, 50)),
            (1.5, Size::new(600, 60)),
            (2.0, Size::new(800, 80)),
        ] {
            let frame = paint_scaled(&mut p, size, scale);
            assert_eq!(frame.size(), size);
            let (x0, y0, x1, y1) = light_bounds(&frame, 0..size.height).expect("hay hora");

            let height = f64::from(y1 - y0);
            let expected = base_height * f64::from(scale);
            assert!(
                (height - expected).abs() <= 2.0,
                "a {scale}x la hora mide {height} px y debería rondar {expected}"
            );
            let center_x = f64::from(x0 + x1) / 2.0;
            let center_y = f64::from(y0 + y1) / 2.0;
            assert!(
                (center_x - f64::from(size.width) / 2.0).abs() <= 4.0,
                "x a {scale}x"
            );
            assert!(
                (center_y - f64::from(size.height) / 2.0).abs() <= 3.0,
                "y a {scale}x"
            );
        }
    }

    #[test]
    fn hover_layout_holds_at_every_scale() {
        let mut p = painter();
        for (scale, size) in [
            (1.25, Size::new(600, 80)),
            (1.5, Size::new(720, 96)),
            (1.75, Size::new(840, 112)),
            (2.0, Size::new(960, 128)),
        ] {
            let frame = paint_scaled(&mut p, size, scale);
            let middle = size.height / 2;
            let upper = light_bounds(&frame, 0..middle).expect("hora arriba");
            let lower = light_bounds(&frame, middle..size.height).expect("fecha abajo");
            assert!(
                upper.3 < lower.1,
                "a {scale}x la hora queda arriba de la fecha"
            );

            let margin_top = f64::from(upper.1);
            let margin_bottom = f64::from(size.height - lower.3);
            assert!(
                (margin_top - margin_bottom).abs() <= 6.0,
                "a {scale}x el bloque está centrado: {margin_top} arriba, {margin_bottom} abajo"
            );
        }
    }

    #[test]
    fn changing_the_scale_drops_the_glyphs_of_the_old_size() {
        let mut p = painter();
        paint_scaled(&mut p, REST, 1.0);
        let at_100 = p.text.cached_glyphs();
        assert_eq!(at_100, 4, "0, 9, : y 5");

        paint_scaled(&mut p, Size::new(800, 80), 2.0);
        assert_eq!(
            p.text.cached_glyphs(),
            4,
            "solo los del 200 %, sin los del 100 %"
        );

        paint_scaled(&mut p, Size::new(800, 80), 2.0);
        assert_eq!(
            p.text.cached_glyphs(),
            4,
            "con la misma escala no se rasteriza de nuevo"
        );
    }

    #[test]
    fn an_invalid_scale_is_treated_as_one_hundred_percent() {
        let mut p = painter();
        let normal = paint_scaled(&mut p, REST, 1.0);
        for scale in [f32::NAN, 0.0, -2.0, f32::INFINITY] {
            assert_eq!(paint_scaled(&mut p, REST, scale), normal, "escala {scale}");
        }
    }

    #[test]
    fn the_date_fades_in_at_scales_other_than_one_hundred_percent() {
        let mut p = painter();
        let rest = paint_scaled(&mut p, Size::new(600, 60), 1.5);
        let middle = paint_scaled(&mut p, Size::new(660, 78), 1.5);
        let full = paint_scaled(&mut p, Size::new(720, 96), 1.5);

        assert!(light_bounds(&rest, 45..60).is_none(), "sin fecha en reposo");
        let faint = brightest(&middle, 54..78);
        let strong = brightest(&full, 54..96);
        assert!(faint > 0 && faint < strong, "{faint} < {strong}");
    }
}
