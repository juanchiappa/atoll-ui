//! Contenido de la isla: la hora y, al pasar el mouse, la fecha.
//!
//! El backend de la plataforma llama a [`IslandPainter`] cada vez que hay que
//! dibujar. Acá se decide qué se ve y dónde; el backend solo lo entrega al SO.

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

/// Tamaño de la hora, en píxeles.
const TIME_PX: f32 = 22.0;

/// Tamaño de la fecha, en píxeles.
const DATE_PX: f32 = 13.0;

/// Altura de las mayúsculas y los dígitos de Inter, como fracción del tamaño (1490 / 2048).
const CAP_HEIGHT: f32 = 0.7275;

/// Separación entre la hora y la fecha, en píxeles.
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
    collapsed: Size,
    expanded: Size,
}

impl IslandPainter {
    /// Crea el painter para una isla que va de `collapsed` a `expanded`.
    ///
    /// # Errors
    ///
    /// Devuelve [`RenderError`] si no se puede cargar la fuente embebida.
    pub fn new(collapsed: Size, expanded: Size) -> Result<Self, RenderError> {
        Ok(Self {
            text: TextRenderer::new()?,
            collapsed,
            expanded,
        })
    }

    /// Qué tan expandida está la isla: 0,0 en reposo y 1,0 con hover.
    fn progress(&self, size: Size) -> f32 {
        let span = self.expanded.height.saturating_sub(self.collapsed.height);
        if span == 0 {
            return 0.0;
        }
        let done = size.height.saturating_sub(self.collapsed.height);
        (done as f32 / span as f32).clamp(0.0, 1.0)
    }

    /// Dibuja un cuadro completo: isla, hora y fecha.
    fn draw(&mut self, size: Size, time: &LocalTime) -> Result<Frame, RenderError> {
        let mut frame = render_island(size)?;
        let progress = self.progress(size);
        let width = size.width as f32;
        let height = size.height as f32;

        let time_cap = TIME_PX * CAP_HEIGHT;
        let date_cap = DATE_PX * CAP_HEIGHT;

        // En reposo la hora va centrada; expandida, hora y fecha forman un bloque centrado.
        // La línea base de la hora se mueve de una posición a la otra junto con la isla.
        let rest_baseline = (height + time_cap) / 2.0;
        let block = time_cap + LINE_GAP + date_cap;
        let expanded_baseline = (height - block) / 2.0 + time_cap;
        let time_baseline = rest_baseline + (expanded_baseline - rest_baseline) * progress;

        let time_text = format_time(time);
        let time_width = self.text.measure(&time_text, TIME_PX)?;
        self.text.draw(
            &mut frame,
            &time_text,
            (width - time_width) / 2.0,
            time_baseline,
            TIME_PX,
            TIME_COLOR,
        )?;

        // La fecha aparece con la expansión: transparente en reposo, visible con hover.
        let alpha = (progress * DATE_ALPHA).round();
        if alpha >= 1.0 {
            let date_text = format_date(time);
            let date_width = self.text.measure(&date_text, DATE_PX)?;
            let color = Color::rgba(255, 255, 255, alpha as u8);
            self.text.draw(
                &mut frame,
                &date_text,
                (width - date_width) / 2.0,
                time_baseline + LINE_GAP + date_cap,
                DATE_PX,
                color,
            )?;
        }
        Ok(frame)
    }
}

impl Painter for IslandPainter {
    fn paint(&mut self, size: Size, time: LocalTime) -> Option<Frame> {
        self.draw(size, &time).ok()
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
        painter.paint(size, time).expect("dibuja")
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
}
