//! Escalado por DPI.
//!
//! Los tamaños de la configuración (`IslandConfig`) son **lógicos**: píxeles a 96 DPI,
//! que es el 100 % de Windows. Para dibujar se multiplican por el factor de escala del
//! monitor donde está la isla (125 % = 1,25), así la isla mide lo mismo en pantalla
//! —y se ve nítida— en cualquier monitor. Todo es aritmética pura, sin código de SO.

use atoll_render::Size;

/// DPI que equivale al 100 % de escala.
pub const BASE_DPI: u32 = 96;

/// Escala mínima aceptada.
const MIN_SCALE: f32 = 0.5;

/// Escala máxima aceptada.
const MAX_SCALE: f32 = 8.0;

/// Factor de escala para un DPI dado (96 → 1,0; 120 → 1,25; 192 → 2,0).
///
/// Un DPI de 0 (el SO no pudo informarlo) se toma como 100 %. El resultado
/// queda acotado entre 0,5 y 8,0 para que un dato absurdo no rompa el dibujo.
#[must_use]
pub fn scale_for_dpi(dpi: u32) -> f32 {
    if dpi == 0 {
        return 1.0;
    }
    (dpi as f32 / BASE_DPI as f32).clamp(MIN_SCALE, MAX_SCALE)
}

/// Escala un valor en píxeles lógicos y lo redondea al píxel físico más cercano.
#[must_use]
pub fn scale_px(value: u32, scale: f32) -> u32 {
    (value as f32 * scale).round().max(0.0) as u32
}

/// Escala un tamaño lógico a píxeles físicos.
#[must_use]
pub fn scale_size(size: Size, scale: f32) -> Size {
    Size::new(scale_px(size.width, scale), scale_px(size.height, scale))
}

#[cfg(test)]
mod tests {
    use super::{scale_for_dpi, scale_px, scale_size};
    use atoll_render::Size;

    #[test]
    fn common_windows_scales_map_to_exact_factors() {
        assert!((scale_for_dpi(96) - 1.0).abs() < f32::EPSILON);
        assert!((scale_for_dpi(120) - 1.25).abs() < f32::EPSILON);
        assert!((scale_for_dpi(144) - 1.5).abs() < f32::EPSILON);
        assert!((scale_for_dpi(168) - 1.75).abs() < f32::EPSILON);
        assert!((scale_for_dpi(192) - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn unknown_or_absurd_dpi_is_contained() {
        assert!((scale_for_dpi(0) - 1.0).abs() < f32::EPSILON);
        assert!((scale_for_dpi(10) - 0.5).abs() < f32::EPSILON);
        assert!((scale_for_dpi(1_000_000) - 8.0).abs() < f32::EPSILON);
    }

    #[test]
    fn pixels_are_rounded_to_the_nearest_whole_pixel() {
        assert_eq!(scale_px(40, 1.25), 50);
        assert_eq!(scale_px(8, 1.5), 12);
        assert_eq!(scale_px(33, 1.25), 41);
        assert_eq!(scale_px(0, 2.0), 0);
    }

    #[test]
    fn invalid_scale_never_produces_negative_or_huge_sizes() {
        assert_eq!(scale_px(40, f32::NAN), 0);
        assert_eq!(scale_px(40, -2.0), 0);
    }

    #[test]
    fn the_island_keeps_the_same_physical_proportions_at_every_scale() {
        let rest = Size::new(400, 40);
        let hover = Size::new(480, 64);
        for (scale, rest_px, hover_px) in [
            (1.0, Size::new(400, 40), Size::new(480, 64)),
            (1.25, Size::new(500, 50), Size::new(600, 80)),
            (1.5, Size::new(600, 60), Size::new(720, 96)),
            (2.0, Size::new(800, 80), Size::new(960, 128)),
        ] {
            assert_eq!(scale_size(rest, scale), rest_px, "reposo a {scale}");
            assert_eq!(scale_size(hover, scale), hover_px, "hover a {scale}");
        }
    }
}
