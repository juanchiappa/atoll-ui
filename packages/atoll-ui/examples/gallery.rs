//! Galería visual de `atoll-ui` (Ítem 14c).
//!
//! Renderiza una pantalla de muestra con todos los widgets en todos sus estados
//! (`normal`, `hover`, `pressed`, `disabled`; `Toggle` encendido y apagado;
//! `Slider` a 0 %, 50 % y 100 %; `ProgressBar` a 0 %, 50 % y 100 %; contenedor
//! `Box` con fondo, `Divider` y los 11 íconos + marcador de ícono desconocido)
//! para los 3 temas (`black`, `graphite`, `white`) y las escalas `1.0` y `1.5`.
//!
//! Uso:
//! ```text
//! cargo run -p atoll-ui --example gallery
//! ```
//! Escribe los archivos `.bmp` de 24 bits en `target/atoll-ui-gallery/` e imprime sus rutas.

use std::error::Error;
use std::fs;
use std::path::Path;

use atoll_config::{Color as ConfigColor, Config, Theme};
use atoll_render::{Frame, Size as RenderSize, TextRenderer};
use atoll_ui::{
    build_gallery_tree, compute_layout, paint_tree_with_text, PointerInput, Rect, Size,
    ThemePalette, UiState, WidgetId,
};

const LOGICAL_WIDTH: u32 = 680;
const LOGICAL_HEIGHT: u32 = 440;

const HOVER_IDS: &[u32] = &[11, 21, 31, 35, 42];
const PRESSED_IDS: &[u32] = &[12, 22, 32, 36];

fn main() -> Result<(), Box<dyn Error>> {
    let out_dir = Path::new("target").join("atoll-ui-gallery");
    fs::create_dir_all(&out_dir)?;

    let mut text_renderer = TextRenderer::new()?;

    let themes = [
        ("black", Theme::Black),
        ("graphite", Theme::Graphite),
        ("white", Theme::White),
    ];
    let scales = [("1.0x", 1.0_f32), ("1.5x", 1.5_f32)];

    for (theme_name, theme) in themes {
        let palette = palette_for_theme(theme);
        for (scale_label, scale) in scales {
            let phys_w = ((LOGICAL_WIDTH as f32) * scale).round() as u32;
            let phys_h = ((LOGICAL_HEIGHT as f32) * scale).round() as u32;
            let avail = Size::new(phys_w, phys_h);

            let tree = build_gallery_tree(theme_name, scale_label, &palette);
            let layout = compute_layout(&tree, avail, scale, &mut text_renderer)?;

            let render_size = RenderSize::new(phys_w, phys_h);
            let bg_render = palette.background.to_render_color();
            let mut frame = Frame::new_filled(render_size, bg_render)?;

            // 1. Pintado base con estado en reposo.
            let state_idle = UiState::new();
            paint_tree_with_text(
                &tree,
                &layout,
                &state_idle,
                &palette,
                scale,
                &mut frame,
                &mut text_renderer,
            );

            // 2. Superponemos la región exacta de los widgets de demostración en estado `hover`.
            for &raw_id in HOVER_IDS {
                let id = WidgetId::new(raw_id);
                if let Some(lnode) = layout.find_by_id(id) {
                    let cx = lnode.rect.x + (lnode.rect.width as i32) / 2;
                    let cy = lnode.rect.y + (lnode.rect.height as i32) / 2;
                    let mut state_hover = UiState::new();
                    let _ = state_hover.handle(&tree, &layout, PointerInput::move_to(cx, cy));
                    let mut tmp = Frame::new_filled(render_size, bg_render)?;
                    paint_tree_with_text(
                        &tree,
                        &layout,
                        &state_hover,
                        &palette,
                        scale,
                        &mut tmp,
                        &mut text_renderer,
                    );
                    copy_rect_pixels(&mut frame, &tmp, lnode.rect);
                }
            }

            // 3. Superponemos la región exacta de los widgets de demostración en estado `pressed`.
            for &raw_id in PRESSED_IDS {
                let id = WidgetId::new(raw_id);
                if let Some(lnode) = layout.find_by_id(id) {
                    let cx = lnode.rect.x + (lnode.rect.width as i32) / 2;
                    let cy = lnode.rect.y + (lnode.rect.height as i32) / 2;
                    let mut state_pressed = UiState::new();
                    let _ = state_pressed.handle(&tree, &layout, PointerInput::left_down(cx, cy));
                    let mut tmp = Frame::new_filled(render_size, bg_render)?;
                    paint_tree_with_text(
                        &tree,
                        &layout,
                        &state_pressed,
                        &palette,
                        scale,
                        &mut tmp,
                        &mut text_renderer,
                    );
                    copy_rect_pixels(&mut frame, &tmp, lnode.rect);
                }
            }

            let file_name = format!("gallery_{theme_name}_{scale_label}.bmp");
            let file_path = out_dir.join(file_name);
            write_bmp_24(&file_path, &frame)?;
            println!("{}", file_path.display());
        }
    }

    Ok(())
}

fn palette_for_theme(theme: Theme) -> ThemePalette {
    let mut cfg = Config::default();
    cfg.appearance.theme = theme;
    match theme {
        Theme::Black => {
            cfg.appearance.background = ConfigColor::rgba(0x00, 0x00, 0x00, 0xFF);
            cfg.appearance.text_color = ConfigColor::rgba(0xFF, 0xFF, 0xFF, 0xFF);
        }
        Theme::Graphite => {
            cfg.appearance.background = ConfigColor::rgba(0x1C, 0x1C, 0x1E, 0xFF);
            cfg.appearance.text_color = ConfigColor::rgba(0xF5, 0xF5, 0xF7, 0xFF);
        }
        Theme::White => {
            cfg.appearance.background = ConfigColor::rgba(0xF5, 0xF5, 0xF7, 0xFF);
            cfg.appearance.text_color = ConfigColor::rgba(0x1C, 0x1C, 0x1E, 0xFF);
        }
    }
    ThemePalette::from_appearance(&cfg.appearance)
}

fn copy_rect_pixels(dst: &mut Frame, src: &Frame, rect: Rect) {
    let fsize = dst.size();
    let bounds = Rect::new(0, 0, fsize.width, fsize.height).intersect(rect);
    if bounds.is_empty() {
        return;
    }
    let stride = fsize.width as usize * 4;
    let x0 = bounds.x as usize;
    let y0 = bounds.y as usize;
    let copy_bytes = bounds.width as usize * 4;
    let src_data = src.rgba_premultiplied();
    let dst_data = dst.rgba_premultiplied_mut();

    for row in 0..(bounds.height as usize) {
        let offset = (y0 + row) * stride + x0 * 4;
        dst_data[offset..offset + copy_bytes]
            .copy_from_slice(&src_data[offset..offset + copy_bytes]);
    }
}

/// Escritor mínimo de BMP de 24 bits sin compresión (`BI_RGB`) y sin crates externos.
fn write_bmp_24(path: &Path, frame: &Frame) -> std::io::Result<()> {
    let w = frame.size().width as usize;
    let h = frame.size().height as usize;
    let row_stride = (w * 3 + 3) & !3;
    let pixel_bytes = row_stride * h;
    let file_size = 54 + pixel_bytes;

    let mut out = Vec::with_capacity(file_size);
    // Cabecera BITMAPFILEHEADER (14 bytes)
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&(file_size as u32).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&54u32.to_le_bytes());
    // Cabecera BITMAPINFOHEADER (40 bytes)
    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&(w as i32).to_le_bytes());
    out.extend_from_slice(&(-(h as i32)).to_le_bytes()); // top-down
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&24u16.to_le_bytes());
    out.extend_from_slice(&[0u8; 24]);

    let rgba = frame.rgba_premultiplied();
    let pad = [0u8; 3];
    for y in 0..h {
        let row_start = y * w * 4;
        for x in 0..w {
            let i = row_start + x * 4;
            out.extend_from_slice(&[rgba[i + 2], rgba[i + 1], rgba[i]]); // BGR
        }
        out.extend_from_slice(&pad[..(row_stride - w * 3)]);
    }
    fs::write(path, out)
}
