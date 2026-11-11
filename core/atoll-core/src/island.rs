//! Contenido de la isla: la hora, la fecha al pasar el mouse y la aplicación de temas.
//!
//! El backend de la plataforma llama a [`IslandPainter`] cada vez que hay que
//! dibujar o cuando cambia `config.ini` en disco. Acá se decide qué se ve y dónde;
//! el backend solo lo entrega al SO.
//!
//! Las medidas de este módulo (tamaño de letra, separaciones, radio de esquina)
//! están en píxeles lógicos, a 96 DPI. Al dibujar se multiplican por el factor de
//! escala del monitor.

use std::fs;
use std::path::{Path, PathBuf};

use atoll_config::{Color as ConfigColor, Config, CornerRadius, Warning};
use atoll_platform::dpi::scale_px;
use atoll_platform::{IslandConfig, LocalTime, Painter};
use atoll_render::{render_island_with, Color, Frame, RenderError, Size, TextRenderer};

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

/// Construye [`IslandConfig`] a partir de [`Config`] y la carpeta opcional de recarga en vivo.
#[must_use]
pub fn island_config_from(config: &Config, config_dir: Option<PathBuf>) -> IslandConfig {
    let base = IslandConfig::new(
        Size::new(config.island.width, config.island.height),
        Size::new(config.island.hover_width, config.island.hover_height),
        config.island.top_margin,
    );
    match config_dir {
        Some(dir) => base.with_config_dir(dir),
        None => base,
    }
}

/// Combina un color de configuración (`#RRGGBB` / `#RRGGBBAA`) con el porcentaje de
/// opacidad general (`0..=100`) y devuelve el [`Color`] efectivo para el render.
#[must_use]
pub fn effective_color(color: ConfigColor, opacity: u8) -> Color {
    let pct = u32::from(opacity.min(100));
    let alpha = ((u32::from(color.a) * pct + 50) / 100).min(255) as u8;
    Color::rgba(color.r, color.g, color.b, alpha)
}

/// Convierte el radio lógico de configuración a píxeles físicos según `scale`.
///
/// Devuelve `None` cuando es [`CornerRadius::Auto`] (píldora).
#[must_use]
pub fn physical_corner_radius(radius: CornerRadius, scale: f32) -> Option<f32> {
    match radius {
        CornerRadius::Auto => None,
        CornerRadius::Px(px) => Some((px as f32) * scale),
    }
}

/// Imprime en `stderr` todas las advertencias producidas por el parser de configuración.
pub fn report_warnings(warnings: &[Warning]) {
    for warning in warnings {
        eprintln!("atoll: config.ini ({warning})");
    }
}

/// Carga `config.ini` desde `file` o lo crea (junto con `dir`) con los valores por defecto.
///
/// Nunca falla: si no se puede crear o leer, usa [`Config::default`]; si tiene advertencias,
/// las escribe en `stderr`.
#[must_use]
pub fn load_or_create_config(dir: &Path, file: &Path) -> Config {
    if !file.exists() {
        if fs::create_dir_all(dir).is_ok() {
            let _ = fs::write(file, Config::default().to_text());
        }
        return Config::default();
    }
    let Ok(text) = fs::read_to_string(file) else {
        return Config::default();
    };
    let (config, warnings) = atoll_config::parse(&text);
    report_warnings(&warnings);
    config
}

/// Dibuja la isla con la hora y la fecha, aplicando el tema y los tamaños de [`Config`].
#[derive(Debug)]
pub struct IslandPainter {
    text: TextRenderer,
    config: Config,
    config_dir: Option<PathBuf>,
    config_file: Option<PathBuf>,
    /// Escala con la que se dibujó el último cuadro (bits del `f32`), para saber cuándo cambia.
    last_scale: u32,
}

impl IslandPainter {
    /// Crea el painter para una isla con tamaños lógicos `collapsed` y `expanded` y el tema por defecto.
    ///
    /// # Errors
    ///
    /// Devuelve [`RenderError`] si no se puede cargar la fuente embebida.
    #[allow(dead_code)]
    pub fn new(collapsed: Size, expanded: Size) -> Result<Self, RenderError> {
        let mut config = Config::default();
        config.island.width = collapsed.width;
        config.island.height = collapsed.height;
        config.island.hover_width = expanded.width;
        config.island.hover_height = expanded.height;
        Self::from_config(config, None, None)
    }

    /// Crea el painter a partir de una [`Config`] completa y las rutas opcionales de recarga en vivo.
    ///
    /// # Errors
    ///
    /// Devuelve [`RenderError`] si no se puede cargar la fuente embebida.
    pub fn from_config(
        config: Config,
        config_dir: Option<PathBuf>,
        config_file: Option<PathBuf>,
    ) -> Result<Self, RenderError> {
        Ok(Self {
            text: TextRenderer::new()?,
            config,
            config_dir,
            config_file,
            last_scale: 1.0_f32.to_bits(),
        })
    }

    /// Configuración actual aplicada en el painter.
    #[must_use]
    #[allow(dead_code)]
    pub const fn config(&self) -> &Config {
        &self.config
    }

    /// Aplica `next` únicamente si es distinta de la [`Config`] actual.
    ///
    /// Si cambió, actualiza la configuración interna y devuelve `Some(IslandConfig)`
    /// con las nuevas medidas lógicas; si es idéntica, devuelve `None`.
    pub fn apply_config_if_changed(&mut self, next: Config) -> Option<IslandConfig> {
        if next == self.config {
            return None;
        }
        self.config = next;
        Some(island_config_from(&self.config, self.config_dir.clone()))
    }

    /// Parsea `text` y aplica la nueva [`Config`] solo si cambió respecto de la actual.
    pub fn reload_from_text(&mut self, text: &str) -> (Option<IslandConfig>, Vec<Warning>) {
        let (parsed, warnings) = atoll_config::parse(text);
        let updated = self.apply_config_if_changed(parsed);
        (updated, warnings)
    }

    /// Qué tan expandida está la isla: 0,0 en reposo y 1,0 con hover.
    ///
    /// `size` es el tamaño físico del cuadro; los tamaños de reposo y expandido se
    /// escalan con `scale` para compararlos en las mismas unidades.
    fn progress(&self, size: Size, scale: f32) -> f32 {
        let collapsed = scale_px(self.config.island.height, scale);
        let expanded = scale_px(self.config.island.hover_height, scale);
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

        let appearance = self.config.appearance;
        let bg_color = effective_color(appearance.background, appearance.opacity);
        let radius = physical_corner_radius(appearance.corner_radius, scale);
        let mut frame = render_island_with(size, bg_color, radius)?;

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

        let text_color = effective_color(appearance.text_color, appearance.opacity);
        let time_text = format_time(time);
        let time_width = self.text.measure(&time_text, time_px)?;
        self.text.draw(
            &mut frame,
            &time_text,
            (width - time_width) / 2.0,
            time_baseline,
            time_px,
            text_color,
        )?;

        // La fecha aparece con la expansión: transparente en reposo, visible con hover.
        let max_date_alpha = f32::from(text_color.a) * (DATE_ALPHA / 255.0);
        let alpha = (progress * max_date_alpha).round();
        if alpha >= 1.0 {
            let date_text = format_date(time);
            let date_width = self.text.measure(&date_text, date_px)?;
            let date_color = Color::rgba(
                text_color.r,
                text_color.g,
                text_color.b,
                (alpha as u32).min(255) as u8,
            );
            self.text.draw(
                &mut frame,
                &date_text,
                (width - date_width) / 2.0,
                time_baseline + line_gap + date_cap,
                date_px,
                date_color,
            )?;
        }
        Ok(frame)
    }
}

impl Painter for IslandPainter {
    fn paint(&mut self, size: Size, scale: f32, time: LocalTime) -> Option<Frame> {
        self.draw(size, scale, &time).ok()
    }

    fn reload_config(&mut self) -> Option<IslandConfig> {
        let path = self.config_file.as_ref()?;
        // Si el archivo está bloqueado, a medio escribir o fue borrado, conservamos la actual.
        let text = fs::read_to_string(path).ok()?;
        let (updated, warnings) = self.reload_from_text(&text);
        report_warnings(&warnings);
        updated
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::{
        effective_color, format_date, format_time, island_config_from, load_or_create_config,
        physical_corner_radius, IslandPainter,
    };
    use atoll_config::{Color as ConfigColor, Config, CornerRadius, Theme};
    use atoll_platform::{LocalTime, Painter};
    use atoll_render::{Color, Frame, Size};

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

    #[test]
    fn maps_config_to_island_config_and_appearance_correctly() {
        let (cfg, warnings) = atoll_config::parse(
            "
[island]
width = 440
height = 48
hover_width = 560
hover_height = 76
top_margin = 14

[appearance]
theme = graphite
opacity = 50
corner_radius = 12
",
        );
        assert!(warnings.is_empty());

        let dir = PathBuf::from("C:\\Users\\test\\AppData\\Roaming\\Atoll");
        let island_cfg = island_config_from(&cfg, Some(dir.clone()));
        assert_eq!(island_cfg.size, Size::new(440, 48));
        assert_eq!(island_cfg.hover_size, Size::new(560, 76));
        assert_eq!(island_cfg.top_margin, 14);
        assert_eq!(island_cfg.config_dir, Some(dir));

        let eff_bg = effective_color(cfg.appearance.background, cfg.appearance.opacity);
        assert_eq!(eff_bg, Color::rgba(0x1C, 0x1C, 0x1E, 128));
        assert_eq!(
            physical_corner_radius(cfg.appearance.corner_radius, 1.5),
            Some(18.0)
        );
        assert_eq!(physical_corner_radius(CornerRadius::Auto, 2.0), None);
    }

    #[test]
    fn custom_appearance_paints_premultiplied_colors_and_radius() {
        let mut appearance = Theme::White.appearance();
        appearance.background = ConfigColor::rgb(200, 100, 50);
        appearance.opacity = 50;
        appearance.corner_radius = CornerRadius::Px(0);
        let cfg = Config {
            appearance,
            ..Config::default()
        };

        let mut p = IslandPainter::from_config(cfg, None, None).expect("painter");
        let frame = paint(&mut p, HOVER, saturday());

        // Con corner_radius = 0, la esquina (0, 0) forma parte del fondo con opacidad 50 %.
        let corner = pixel(&frame, 0, 0);
        assert_eq!(corner[3], 128);
        assert!((i16::from(corner[0]) - 100).abs() <= 1);
        assert!((i16::from(corner[1]) - 50).abs() <= 1);
        assert!((i16::from(corner[2]) - 25).abs() <= 1);

        // Todos los píxeles del cuadro mantienen alfa premultiplicado válido (c <= a).
        let (pixels, _) = frame.rgba_premultiplied().as_chunks::<4>();
        for &[r, g, b, a] in pixels {
            assert!(
                r <= a && g <= a && b <= a,
                "premultiplicado inválido: {r},{g},{b},{a}"
            );
        }
    }

    #[test]
    fn reload_applies_only_when_config_actually_changed() {
        let mut p = painter();
        let same_text = Config::default().to_text();
        let (unchanged, w1) = p.reload_from_text(&same_text);
        assert!(w1.is_empty());
        assert!(
            unchanged.is_none(),
            "si la configuración es igual no debe pedir redibujar"
        );

        let changed_text =
            "[island]\nwidth = 440\nhover_width = 520\n[appearance]\ntheme = white\n";
        let (updated, w2) = p.reload_from_text(changed_text);
        assert!(w2.is_empty());
        let new_island_cfg = updated.expect("cambió la configuración");
        assert_eq!(new_island_cfg.size, Size::new(440, 40));
        assert_eq!(new_island_cfg.hover_size, Size::new(520, 64));
        assert_eq!(p.config().appearance.theme, Theme::White);

        // Reaplicar exactamente el mismo texto vuelve a dar `None`.
        let (again, w3) = p.reload_from_text(changed_text);
        assert!(w3.is_empty());
        assert!(again.is_none());
    }

    #[test]
    fn missing_or_deleted_config_file_is_handled_gracefully() {
        let mut temp_dir = std::env::temp_dir();
        temp_dir.push(format!("atoll_test_config_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        let file = temp_dir.join("config.ini");

        // Si no existe, `load_or_create_config` lo crea con los valores por defecto.
        let initial = load_or_create_config(&temp_dir, &file);
        assert_eq!(initial, Config::default());
        assert!(file.exists());

        let mut p = IslandPainter::from_config(initial, Some(temp_dir.clone()), Some(file.clone()))
            .expect("painter");

        // Editamos el archivo en disco y recargamos.
        fs::write(&file, "[appearance]\ntheme = graphite\n").expect("escribe");
        assert!(p.reload_config().is_some());
        assert_eq!(p.config().appearance.theme, Theme::Graphite);

        // Si se borra el archivo con la isla abierta, `reload_config` conserva lo último aplicado.
        fs::remove_file(&file).expect("borra");
        assert!(p.reload_config().is_none());
        assert_eq!(p.config().appearance.theme, Theme::Graphite);

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
