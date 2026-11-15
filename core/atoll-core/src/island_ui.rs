//! Interfaz de widgets (`atoll-ui`) de la isla expandida.
//!
//! Este módulo es 100 % portable (sin llamadas a Windows) y concentra:
//! - Los identificadores constantes de los botones ([`WIDGET_ID_SETTINGS`] y [`WIDGET_ID_CLOSE`]).
//! - La construcción pura del árbol de widgets para la isla expandida ([`build_expanded_tree`] y
//!   [`build_expanded_tree_styled`]).
//! - El mapeo entre los eventos neutros de `atoll-platform` ([`PointerEvent`], [`MouseButton`],
//!   [`PainterAction`]) y los tipos de `atoll-ui` ([`PointerInput`], [`PointerButton`], [`UiEvent`]).
//! - El estado interactivo y el layout cacheado ([`ExpandedIslandUi`]) que viven dentro de
//!   [`crate::island::IslandPainter`].

use atoll_config::{Config, CornerRadius, IslandStyle};
use atoll_platform::dpi::scale_size;
use atoll_platform::{MouseButton, PainterAction, PointerEvent, PointerOutcome};
use atoll_render::{Frame, Size, TextRenderer};
use atoll_ui::{
    compute_layout, paint_tree_with_text, ButtonVariant, CrossAlign, Edges, IconId, LayoutTree,
    Length, MainAlign, Node, PointerButton, PointerInput, Size as UiSize, ThemePalette, Tone,
    TreeError, UiEvent, UiState, WidgetId,
};

/// Identificador estable del botón de Ajustes (engranaje).
pub const WIDGET_ID_SETTINGS: WidgetId = WidgetId::new(1);

/// Identificador estable del botón de Cerrar (`X`).
pub const WIDGET_ID_CLOSE: WidgetId = WidgetId::new(2);

/// Tamaño lógico de la fuente de la hora cuando la isla está expandida (`hover_height >= 56`).
const EXPANDED_TIME_PX: u16 = 20;

/// Tamaño lógico de la fuente de la fecha cuando la isla está expandida (`hover_height >= 56`).
const EXPANDED_DATE_PX: u16 = 13;

/// Separación vertical lógica entre la hora y la fecha en la columna izquierda.
const TEXT_COLUMN_GAP_PX: u16 = 4;

/// Separación horizontal lógica entre los botones de Ajustes y Cerrar.
const BUTTONS_GAP_PX: u16 = 6;

/// Separación horizontal mínima entre el bloque de texto y el bloque de botones.
const MAIN_ROW_GAP_PX: u16 = 8;

/// Convierte el tamaño lógico expandido (`hover_size`) al tamaño físico disponible para
/// `compute_layout`, usando exactamente la misma función [`scale_size`] que define el
/// tamaño físico de la ventana en `atoll-platform`.
#[must_use]
pub fn physical_hover_size(hover_size: Size, scale: f32) -> UiSize {
    let phys = scale_size(hover_size, scale);
    UiSize::new(phys.width, phys.height)
}

/// Calcula los márgenes internos lógicos (`Edges`) para que el contenido de la isla
/// expandida respete el redondeo de las esquinas y el estilo (`island` o `notch`) sin
/// quedar pegado al borde ni recortado.
#[must_use]
pub fn expanded_padding(
    logical_size: Size,
    corner_radius: CornerRadius,
    style: IslandStyle,
) -> Edges {
    let w = u16::try_from(logical_size.width).unwrap_or(u16::MAX);
    let h = u16::try_from(logical_size.height).unwrap_or(u16::MAX);
    let max_r = w.min(h) / 2;
    let eff_r = match corner_radius {
        CornerRadius::Auto => max_r,
        CornerRadius::Px(px) => u16::try_from(px).unwrap_or(u16::MAX).min(max_r),
    };

    // Margen horizontal base (14 px) más un resguardo proporcional al radio de esquina,
    // acotado para no consumir más de un sexto del ancho en islas angostas.
    let pad_x = (14_u16.saturating_add(eff_r / 3)).min(w / 6).max(8);

    // Margen vertical adaptado al alto lógico disponible y al estilo (`island` o `notch`).
    let base_pad_y = if h >= 56 {
        6
    } else if h >= 44 {
        3
    } else {
        1
    };
    let (pad_top, pad_bottom) = match style {
        IslandStyle::Island => (base_pad_y, base_pad_y),
        // En modo `notch` el borde superior es recto contra y = 0 y las esquinas inferiores
        // son redondeadas: mantenemos el mismo total vertical para centrar el contenido y
        // resguardar las esquinas inferiores.
        IslandStyle::Notch => (base_pad_y, base_pad_y),
    };

    Edges {
        top: pad_top,
        right: pad_x,
        bottom: pad_bottom,
        left: pad_x,
    }
}

/// Construye el árbol declarativo de widgets de la isla expandida con el estilo y radio por defecto.
///
/// Es una función pura: recibe el texto de la hora, el texto de la fecha y el tamaño lógico
/// de la isla expandida (`logical_size`).
#[cfg(test)]
#[must_use]
pub fn build_expanded_tree(time_text: &str, date_text: &str, logical_size: Size) -> Node {
    build_expanded_tree_styled(
        time_text,
        date_text,
        logical_size,
        CornerRadius::Auto,
        IslandStyle::Island,
    )
}

/// Construye el árbol declarativo de widgets de la isla expandida contemplando el radio de esquina
/// y el estilo (`island` o `notch`).
///
/// A la izquierda ubica una columna con la hora (`Label` grande, `Tone::Primary`) arriba y la
/// fecha (`Label` chico, `Tone::Muted`) abajo; a la derecha ubica una fila con el `IconButton`
/// de Ajustes ([`WIDGET_ID_SETTINGS`], engranaje) y el `IconButton` de Cerrar
/// ([`WIDGET_ID_CLOSE`], `X`).
#[must_use]
pub fn build_expanded_tree_styled(
    time_text: &str,
    date_text: &str,
    logical_size: Size,
    corner_radius: CornerRadius,
    style: IslandStyle,
) -> Node {
    let h = logical_size.height;
    let (time_px, date_px, col_gap) = if h >= 56 {
        (EXPANDED_TIME_PX, EXPANDED_DATE_PX, TEXT_COLUMN_GAP_PX)
    } else if h >= 42 {
        (15, 11, 2)
    } else {
        (12, 9, 0)
    };

    let left_column = Node::column(
        col_gap,
        MainAlign::Center,
        CrossAlign::Start,
        vec![
            Node::label(time_text, time_px, Tone::Primary),
            Node::label(date_text, date_px, Tone::Muted),
        ],
    );

    let right_buttons = Node::row(
        BUTTONS_GAP_PX,
        MainAlign::End,
        CrossAlign::Center,
        vec![
            Node::icon_button(
                WIDGET_ID_SETTINGS,
                IconId::Settings,
                ButtonVariant::Ghost,
                true,
            ),
            Node::icon_button(WIDGET_ID_CLOSE, IconId::Close, ButtonVariant::Ghost, true),
        ],
    );

    let padding = expanded_padding(logical_size, corner_radius, style);

    Node::row(
        MAIN_ROW_GAP_PX,
        MainAlign::SpaceBetween,
        CrossAlign::Center,
        vec![left_column, right_buttons],
    )
    .with_padding(padding)
    .with_width(Length::Fill(1))
    .with_height(Length::Fill(1))
}

/// Traduce un [`MouseButton`] de `atoll-platform` al [`PointerButton`] de `atoll-ui`.
#[must_use]
pub const fn map_mouse_button(button: MouseButton) -> PointerButton {
    match button {
        MouseButton::Left => PointerButton::Left,
        MouseButton::Right => PointerButton::Right,
        MouseButton::Middle => PointerButton::Middle,
        MouseButton::Other(code) => PointerButton::Other(code),
    }
}

/// Traduce un [`PointerEvent`] de `atoll-platform` (en píxeles físicos) al [`PointerInput`]
/// de `atoll-ui` (también en píxeles físicos, sin dividir por la escala).
#[must_use]
pub const fn map_pointer_event(event: PointerEvent) -> PointerInput {
    match event {
        PointerEvent::Move { x, y } => PointerInput::Move { x, y },
        PointerEvent::Leave => PointerInput::Leave,
        PointerEvent::Down { x, y, button } => PointerInput::Down {
            x,
            y,
            button: map_mouse_button(button),
        },
        PointerEvent::Up { x, y, button } => PointerInput::Up {
            x,
            y,
            button: map_mouse_button(button),
        },
    }
}

/// Traduce un [`UiEvent`] emitido por `atoll-ui` a una [`PainterAction`] de `atoll-platform`.
#[must_use]
pub const fn map_ui_event(event: UiEvent) -> Option<PainterAction> {
    match event {
        UiEvent::Clicked(id) if id.0 == WIDGET_ID_SETTINGS.0 => Some(PainterAction::OpenConfig),
        UiEvent::Clicked(id) if id.0 == WIDGET_ID_CLOSE.0 => Some(PainterAction::Quit),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CachedLayout {
    tree: Node,
    layout: LayoutTree,
    time_text: String,
    date_text: String,
    hover_size: Size,
    scale_bits: u32,
    corner_radius: CornerRadius,
    style: IslandStyle,
}

/// Estado del árbol de widgets, layout físico y [`UiState`] de la isla expandida.
#[derive(Debug, Clone, Default)]
pub struct ExpandedIslandUi {
    ui_state: UiState,
    cached: Option<CachedLayout>,
}

impl ExpandedIslandUi {
    /// Crea el contenedor de estado interactivo vacío.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            ui_state: UiState::new(),
            cached: None,
        }
    }

    /// Devuelve una referencia al [`UiState`] actual.
    #[cfg(test)]
    #[must_use]
    pub const fn ui_state(&self) -> &UiState {
        &self.ui_state
    }

    /// Devuelve el [`LayoutTree`] físico actual, si ya fue calculado.
    #[must_use]
    pub fn layout(&self) -> Option<&LayoutTree> {
        self.cached.as_ref().map(|c| &c.layout)
    }

    /// Reinicia el estado de interacción (por ejemplo cuando la isla deja de estar expandida).
    pub fn reset_interaction(&mut self) {
        self.ui_state = UiState::new();
    }

    /// Asegura que el árbol y el layout estén actualizados para `(config, scale, time_text, date_text)`.
    ///
    /// Si alguno de esos parámetros cambió (cambio de minuto, recarga de `config.ini` o cambio de
    /// DPI), reconstruye el árbol, recalcula el layout físico con `available = scale_size(hover_size, scale)`
    /// y sincroniza [`UiState::sync`]. Devuelve `true` si `UiState::sync` indicó cambio visual.
    ///
    /// # Errors
    ///
    /// Devuelve [`TreeError`] si la validación del árbol fallara.
    pub fn sync_tree(
        &mut self,
        config: &Config,
        scale: f32,
        time_text: &str,
        date_text: &str,
        text_renderer: &mut TextRenderer,
    ) -> Result<bool, TreeError> {
        let hover_size = Size::new(config.island.hover_width, config.island.hover_height);
        let scale_bits = scale.to_bits();
        let corner_radius = config.appearance.corner_radius;
        let style = config.behavior.style;

        let up_to_date = self.cached.as_ref().is_some_and(|c| {
            c.hover_size == hover_size
                && c.scale_bits == scale_bits
                && c.corner_radius == corner_radius
                && c.style == style
                && c.time_text == time_text
                && c.date_text == date_text
        });

        if up_to_date {
            return Ok(false);
        }

        let tree =
            build_expanded_tree_styled(time_text, date_text, hover_size, corner_radius, style);
        let available = physical_hover_size(hover_size, scale);
        let layout = compute_layout(&tree, available, scale, text_renderer)?;
        let sync_redraw = self.ui_state.sync(&tree, &layout);

        self.cached = Some(CachedLayout {
            tree,
            layout,
            time_text: time_text.to_owned(),
            date_text: date_text.to_owned(),
            hover_size,
            scale_bits,
            corner_radius,
            style,
        });

        Ok(sync_redraw)
    }

    /// Dibuja el árbol de la isla expandida sobre `frame`, sincronizando primero el layout si cambió
    /// la hora, la configuración o la escala.
    ///
    /// # Errors
    ///
    /// Devuelve [`TreeError`] si el árbol no pasa la validación de `atoll-ui`.
    pub fn paint(
        &mut self,
        config: &Config,
        scale: f32,
        time_text: &str,
        date_text: &str,
        frame: &mut Frame,
        text_renderer: &mut TextRenderer,
    ) -> Result<(), TreeError> {
        let _ = self.sync_tree(config, scale, time_text, date_text, text_renderer)?;
        if let Some(cached) = &self.cached {
            let palette = ThemePalette::from_appearance(&config.appearance);
            paint_tree_with_text(
                &cached.tree,
                &cached.layout,
                &self.ui_state,
                &palette,
                scale,
                frame,
                text_renderer,
            );
        }
        Ok(())
    }

    /// Procesa un [`PointerEvent`] físico cuando la isla está completamente expandida.
    ///
    /// Si la isla no está expandida (`is_expanded == false`), limpia cualquier estado de
    /// interacción ante `Leave` y devuelve [`PointerOutcome::NONE`] sin redibujar ni emitir acciones.
    #[must_use]
    pub fn handle_pointer(&mut self, event: PointerEvent, is_expanded: bool) -> PointerOutcome {
        if !is_expanded {
            if matches!(event, PointerEvent::Leave) {
                self.reset_interaction();
            }
            return PointerOutcome::NONE;
        }
        let Some(cached) = &self.cached else {
            return PointerOutcome::NONE;
        };

        let input = map_pointer_event(event);
        let outcome = self.ui_state.handle(&cached.tree, &cached.layout, input);
        let mut actions = Vec::new();
        for ev in outcome.events {
            if let Some(action) = map_ui_event(ev) {
                actions.push(action);
            }
        }
        PointerOutcome {
            needs_redraw: outcome.needs_redraw,
            actions,
        }
    }
}

#[cfg(test)]
mod tests {
    use atoll_config::{Color as ConfigColor, Config, CornerRadius, IslandStyle, Theme};
    use atoll_platform::dpi::scale_size;
    use atoll_platform::{LocalTime, MouseButton, Painter, PainterAction, PointerEvent};
    use atoll_render::{Frame, Size, TextRenderer};
    use atoll_ui::{
        compute_layout, validate_tree, IconId, PointerButton, PointerInput, Rect, Tone, UiEvent,
        WidgetId, WidgetKind,
    };

    use super::{
        build_expanded_tree, build_expanded_tree_styled, map_mouse_button, map_pointer_event,
        map_ui_event, physical_hover_size, WIDGET_ID_CLOSE, WIDGET_ID_SETTINGS,
    };
    use crate::island::{
        effective_color, format_date, format_time, IslandPainter, MONTHS, WEEKDAYS,
    };

    const REST: Size = Size::new(400, 40);
    const HOVER: Size = Size::new(480, 64);

    fn sample_time() -> LocalTime {
        LocalTime {
            year: 2026,
            month: 11,
            day: 14,
            weekday: 6,
            hour: 21,
            minute: 7,
            second: 0,
            millisecond: 0,
        }
    }

    fn pixel(frame: &Frame, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * frame.size().width + x) * 4) as usize;
        let px = &frame.rgba_premultiplied()[i..i + 4];
        [px[0], px[1], px[2], px[3]]
    }

    #[test]
    fn tree_has_stable_ids_expected_structure_and_fits_in_480x64_and_custom_sizes() {
        assert_eq!(WIDGET_ID_SETTINGS, WidgetId::new(1));
        assert_eq!(WIDGET_ID_CLOSE, WidgetId::new(2));

        let tree = build_expanded_tree("21:07", "sáb 14 nov", HOVER);
        let stats = validate_tree(&tree).expect("árbol válido");
        assert_eq!(stats.node_count, 7);

        let children = match &tree.kind {
            WidgetKind::Row { children, .. } => children.as_slice(),
            _ => &[],
        };
        assert_eq!(children.len(), 2);

        // Columna izquierda: hora (Primary) y fecha (Muted).
        let left_children = match &children[0].kind {
            WidgetKind::Column { children, .. } => children.as_slice(),
            _ => &[],
        };
        assert_eq!(left_children.len(), 2);
        assert!(matches!(
            &left_children[0].kind,
            WidgetKind::Label {
                text,
                tone: Tone::Primary,
                ..
            } if text == "21:07"
        ));
        assert!(matches!(
            &left_children[1].kind,
            WidgetKind::Label {
                text,
                tone: Tone::Muted,
                ..
            } if text == "sáb 14 nov"
        ));

        // Fila derecha: IconButton de Ajustes y Cerrar.
        let right_children = match &children[1].kind {
            WidgetKind::Row { children, .. } => children.as_slice(),
            _ => &[],
        };
        assert_eq!(right_children.len(), 2);
        assert_eq!(right_children[0].id, WIDGET_ID_SETTINGS);
        assert!(matches!(
            &right_children[0].kind,
            WidgetKind::Button {
                label: None,
                icon: Some(IconId::Settings),
                enabled: true,
                ..
            }
        ));
        assert_eq!(right_children[1].id, WIDGET_ID_CLOSE);
        assert!(matches!(
            &right_children[1].kind,
            WidgetKind::Button {
                label: None,
                icon: Some(IconId::Close),
                enabled: true,
                ..
            }
        ));

        // Debe entrar sin recortes a cero en 480x64 y en distintos tamaños configurados.
        let mut tr = TextRenderer::new().expect("fuente");
        for size in [
            Size::new(480, 64),
            Size::new(320, 48),
            Size::new(400, 40),
            Size::new(600, 80),
            Size::new(720, 96),
        ] {
            let t = build_expanded_tree("21:07", "sáb 14 nov", size);
            let avail = physical_hover_size(size, 1.0);
            let layout = compute_layout(&t, avail, 1.0, &mut tr).expect("layout");
            let settings_node = layout
                .find_by_id(WIDGET_ID_SETTINGS)
                .expect("existe botón Settings");
            let close_node = layout
                .find_by_id(WIDGET_ID_CLOSE)
                .expect("existe botón Close");
            assert!(
                !settings_node.rect.is_empty() && !close_node.rect.is_empty(),
                "botones con área no vacía en {size:?}"
            );
            assert!(
                settings_node.rect.right() <= close_node.rect.x,
                "Settings a la izquierda de Close en {size:?}"
            );
            for node in &layout.nodes {
                assert!(
                    !node.rect.is_empty(),
                    "ningún nodo queda vacío en {size:?}: {node:?}"
                );
            }
        }
    }

    #[test]
    fn layout_root_rect_matches_physical_expanded_window_size_at_all_scales() {
        let mut tr = TextRenderer::new().expect("fuente");
        for hover_size in [Size::new(480, 64), Size::new(540, 72)] {
            for scale in [1.0_f32, 1.25, 1.5, 1.75, 2.0] {
                let expected_win_phys = scale_size(hover_size, scale);
                let available = physical_hover_size(hover_size, scale);
                assert_eq!(available.width, expected_win_phys.width);
                assert_eq!(available.height, expected_win_phys.height);

                let tree = build_expanded_tree("09:05", "sáb 7 nov", hover_size);
                let layout = compute_layout(&tree, available, scale, &mut tr).expect("layout");
                assert_eq!(
                    layout.nodes[0].rect,
                    Rect::new(0, 0, expected_win_phys.width, expected_win_phys.height),
                    "el rectángulo raíz del layout debe coincidir con el tamaño físico de la ventana a {scale}x"
                );
            }
        }
    }

    #[test]
    fn maps_pointer_events_and_ui_events_accurately() {
        assert_eq!(map_mouse_button(MouseButton::Left), PointerButton::Left);
        assert_eq!(map_mouse_button(MouseButton::Right), PointerButton::Right);
        assert_eq!(map_mouse_button(MouseButton::Middle), PointerButton::Middle);
        assert_eq!(
            map_mouse_button(MouseButton::Other(4)),
            PointerButton::Other(4)
        );

        assert_eq!(
            map_pointer_event(PointerEvent::Move { x: 120, y: 34 }),
            PointerInput::Move { x: 120, y: 34 }
        );
        assert_eq!(map_pointer_event(PointerEvent::Leave), PointerInput::Leave);
        assert_eq!(
            map_pointer_event(PointerEvent::Down {
                x: -5,
                y: 10,
                button: MouseButton::Left,
            }),
            PointerInput::Down {
                x: -5,
                y: 10,
                button: PointerButton::Left,
            }
        );
        assert_eq!(
            map_pointer_event(PointerEvent::Up {
                x: 400,
                y: 50,
                button: MouseButton::Right,
            }),
            PointerInput::Up {
                x: 400,
                y: 50,
                button: PointerButton::Right,
            }
        );

        assert_eq!(
            map_ui_event(UiEvent::Clicked(WIDGET_ID_SETTINGS)),
            Some(PainterAction::OpenConfig)
        );
        assert_eq!(
            map_ui_event(UiEvent::Clicked(WIDGET_ID_CLOSE)),
            Some(PainterAction::Quit)
        );
        assert_eq!(map_ui_event(UiEvent::Clicked(WidgetId::new(99))), None);
    }

    #[test]
    fn clicking_button_centers_at_scales_1_0_1_5_and_2_0_emits_actions_and_outside_or_release_outside_does_nothing(
    ) {
        for scale in [1.0_f32, 1.5, 2.0] {
            let mut painter = IslandPainter::new(REST, HOVER).expect("painter");
            painter.set_expanded(true);
            let phys_size = scale_size(HOVER, scale);
            let _ = painter
                .paint(phys_size, scale, sample_time())
                .expect("dibuja expandida");

            let layout = painter.ui().layout().expect("layout calculado").clone();
            let settings_rect = layout
                .find_by_id(WIDGET_ID_SETTINGS)
                .expect("botón Ajustes")
                .rect;
            let close_rect = layout
                .find_by_id(WIDGET_ID_CLOSE)
                .expect("botón Cerrar")
                .rect;

            let settings_cx = settings_rect.x + (settings_rect.width as i32) / 2;
            let settings_cy = settings_rect.y + (settings_rect.height as i32) / 2;
            let close_cx = close_rect.x + (close_rect.width as i32) / 2;
            let close_cy = close_rect.y + (close_rect.height as i32) / 2;

            // 1. Clic completo en el centro físico de Ajustes -> OpenConfig.
            let down_settings = painter.on_pointer(PointerEvent::Down {
                x: settings_cx,
                y: settings_cy,
                button: MouseButton::Left,
            });
            assert!(down_settings.needs_redraw);
            assert!(down_settings.actions.is_empty());

            let up_settings = painter.on_pointer(PointerEvent::Up {
                x: settings_cx,
                y: settings_cy,
                button: MouseButton::Left,
            });
            assert!(up_settings.needs_redraw);
            assert_eq!(
                up_settings.actions,
                vec![PainterAction::OpenConfig],
                "clic en Ajustes a escala {scale}x debe producir OpenConfig"
            );

            // 2. Clic completo en el centro físico de Cerrar -> Quit.
            let down_close = painter.on_pointer(PointerEvent::Down {
                x: close_cx,
                y: close_cy,
                button: MouseButton::Left,
            });
            assert!(down_close.needs_redraw);
            assert!(down_close.actions.is_empty());

            let up_close = painter.on_pointer(PointerEvent::Up {
                x: close_cx,
                y: close_cy,
                button: MouseButton::Left,
            });
            assert!(up_close.needs_redraw);
            assert_eq!(
                up_close.actions,
                vec![PainterAction::Quit],
                "clic en Cerrar a escala {scale}x debe producir Quit"
            );

            // 3. Clic apenas afuera de cada botón no produce ninguna acción.
            for (ox, oy) in [
                (settings_rect.x - 1, settings_cy),
                (settings_rect.right(), settings_cy),
                (settings_cx, settings_rect.y - 1),
                (settings_cx, settings_rect.bottom()),
                (close_rect.x - 1, close_cy),
                (close_rect.right(), close_cy),
                (close_cx, close_rect.y - 1),
                (close_cx, close_rect.bottom()),
            ] {
                let _ = painter.on_pointer(PointerEvent::Down {
                    x: ox,
                    y: oy,
                    button: MouseButton::Left,
                });
                let out = painter.on_pointer(PointerEvent::Up {
                    x: ox,
                    y: oy,
                    button: MouseButton::Left,
                });
                assert!(
                    out.actions.is_empty(),
                    "clic fuera ({ox}, {oy}) a escala {scale}x no debe producir acción"
                );
            }

            // 4. Presionar adentro de Cerrar y soltar afuera (incluso fuera de la ventana) no produce acción.
            let _ = painter.on_pointer(PointerEvent::Down {
                x: close_cx,
                y: close_cy,
                button: MouseButton::Left,
            });
            let move_out = painter.on_pointer(PointerEvent::Move { x: -20, y: -10 });
            assert!(
                move_out.needs_redraw,
                "salir con el botón presionado actualiza el estado visual"
            );
            let release_out = painter.on_pointer(PointerEvent::Up {
                x: -20,
                y: -10,
                button: MouseButton::Left,
            });
            assert!(
                release_out.actions.is_empty(),
                "soltar fuera de la ventana no debe ejecutar la acción"
            );
        }
    }

    #[test]
    fn unchanged_mouse_movement_does_not_request_redraw() {
        let mut painter = IslandPainter::new(REST, HOVER).expect("painter");
        painter.set_expanded(true);
        let _ = painter.paint(HOVER, 1.0, sample_time()).expect("dibuja");

        // Moverse sobre zona vacía sin hover previo -> needs_redraw = false.
        let m1 = painter.on_pointer(PointerEvent::Move { x: 200, y: 32 });
        assert!(!m1.needs_redraw);
        let m2 = painter.on_pointer(PointerEvent::Move { x: 210, y: 32 });
        assert!(!m2.needs_redraw);

        let settings_rect = painter
            .ui()
            .layout()
            .expect("layout")
            .find_by_id(WIDGET_ID_SETTINGS)
            .expect("Ajustes")
            .rect;
        let cx = settings_rect.x + (settings_rect.width as i32) / 2;
        let cy = settings_rect.y + (settings_rect.height as i32) / 2;

        // Entrar al botón de Ajustes -> cambia hover -> needs_redraw = true.
        let enter = painter.on_pointer(PointerEvent::Move { x: cx, y: cy });
        assert!(enter.needs_redraw);

        // Moverse dentro del mismo botón de Ajustes -> no cambia nada -> needs_redraw = false.
        let inside = painter.on_pointer(PointerEvent::Move { x: cx + 1, y: cy });
        assert!(!inside.needs_redraw);

        // Salir a zona vacía -> cambia hover -> needs_redraw = true.
        let leave_btn = painter.on_pointer(PointerEvent::Move { x: 200, y: 32 });
        assert!(leave_btn.needs_redraw);
    }

    #[test]
    fn tree_is_hidden_when_collapsed_or_when_animation_has_not_finished_and_compares_pixels() {
        let mut painter = IslandPainter::new(REST, HOVER).expect("painter");

        // 1. En reposo (400x40): no hay árbol y los eventos de puntero se ignoran.
        painter.set_expanded(false);
        let frame_rest = painter.paint(REST, 1.0, sample_time()).expect("reposo");
        let out_rest = painter.on_pointer(PointerEvent::Move { x: 400, y: 32 });
        assert!(!out_rest.needs_redraw);

        // 2. En tamaño expandido (480x64) pero con `set_expanded(false)` (animación aún no terminada
        // o empezando a contraer): dibuja SOLO la hora centrada, sin fecha ni botones.
        let frame_animating_at_hover_size = painter
            .paint(HOVER, 1.0, sample_time())
            .expect("animando en 480x64");

        // 3. Con `set_expanded(true)` (hover + animación terminada): dibuja el árbol completo.
        painter.set_expanded(true);
        let frame_expanded = painter
            .paint(HOVER, 1.0, sample_time())
            .expect("expandida quieta");

        assert_ne!(
            frame_animating_at_hover_size, frame_expanded,
            "el cuadro con hover completo debe diferir del cuadro cuando la animación no terminó"
        );

        // En `frame_animating_at_hover_size`, la zona derecha donde viven los botones (x: 380..455, y: 18..46)
        // solo tiene el fondo negro (r == 0), mientras que en `frame_expanded` están pintados los íconos (r > 0).
        let frame_animating_ref = &frame_animating_at_hover_size;
        let right_lit_animating = (18..46)
            .flat_map(|y| (380..455).map(move |x| pixel(frame_animating_ref, x, y)[0]))
            .max()
            .unwrap_or(0);
        let frame_expanded_ref = &frame_expanded;
        let right_lit_expanded = (18..46)
            .flat_map(|y| (380..455).map(move |x| pixel(frame_expanded_ref, x, y)[0]))
            .max()
            .unwrap_or(0);
        assert_eq!(
            right_lit_animating, 0,
            "sin terminar la animación no se pintan los botones a la derecha"
        );
        assert!(
            right_lit_expanded > 0,
            "con la expansión terminada se pintan los botones a la derecha"
        );
        assert_eq!(frame_rest.size(), REST);
    }

    #[test]
    fn time_change_config_reload_and_dpi_change_rebuild_tree_and_sync_ui_state() {
        let mut painter = IslandPainter::new(REST, HOVER).expect("painter");
        painter.set_expanded(true);
        let t1 = sample_time();
        let _ = painter.paint(HOVER, 1.0, t1).expect("dibuja t1");

        let settings_rect_1 = painter
            .ui()
            .layout()
            .expect("layout")
            .find_by_id(WIDGET_ID_SETTINGS)
            .expect("Ajustes")
            .rect;
        let cx = settings_rect_1.x + (settings_rect_1.width as i32) / 2;
        let cy = settings_rect_1.y + (settings_rect_1.height as i32) / 2;

        // Ponemos el botón Ajustes en hover.
        let _ = painter.on_pointer(PointerEvent::Move { x: cx, y: cy });
        assert_eq!(painter.ui().ui_state().hovered(), Some(WIDGET_ID_SETTINGS));

        // 1. Cambia el minuto -> se reconstruye el árbol y se mantiene sincronizado el id estable.
        let mut t2 = t1;
        t2.minute = 8;
        let _ = painter.paint(HOVER, 1.0, t2).expect("dibuja t2");
        assert_eq!(painter.ui().ui_state().hovered(), Some(WIDGET_ID_SETTINGS));

        // 2. Recarga de configuración (nuevo hover_width y tema) -> reconstruye y llama a sync.
        let (updated, warnings) = painter
            .reload_from_text("[island]\nhover_width = 560\n[appearance]\ntheme = graphite\n");
        assert!(warnings.is_empty());
        assert!(updated.is_some());
        let settings_rect_reloaded = painter
            .ui()
            .layout()
            .expect("layout tras reload")
            .find_by_id(WIDGET_ID_SETTINGS)
            .expect("Ajustes")
            .rect;
        assert!(
            settings_rect_reloaded.x > settings_rect_1.x,
            "al ensanchar hover_width a 560, el botón de la derecha se desplaza"
        );
        assert_eq!(painter.ui().ui_state().hovered(), Some(WIDGET_ID_SETTINGS));

        // 3. Cambio de DPI (escala 1.5x) -> recalcula layout físico y sincroniza.
        let phys_15 = scale_size(Size::new(560, 64), 1.5);
        let _ = painter.paint(phys_15, 1.5, t2).expect("dibuja 1.5x");
        assert_eq!(
            painter.ui().layout().expect("layout 1.5x").available.width,
            phys_15.width
        );
        assert_eq!(painter.ui().ui_state().hovered(), Some(WIDGET_ID_SETTINGS));
    }

    #[test]
    fn notch_and_island_styles_and_theme_colors_render_cleanly_inside_corners() {
        for style in [IslandStyle::Island, IslandStyle::Notch] {
            for theme in [Theme::Black, Theme::Graphite, Theme::White] {
                let mut cfg = Config::default();
                cfg.behavior.style = style;
                cfg.appearance = theme.appearance();
                cfg.appearance.corner_radius = CornerRadius::Auto;

                let mut painter = IslandPainter::from_config(cfg, None, None).expect("painter");
                painter.set_expanded(true);
                let frame = painter.paint(HOVER, 1.0, sample_time()).expect("dibuja");

                // Esquinas inferiores son siempre redondeadas y transparentes en `CornerRadius::Auto`.
                assert_eq!(pixel(&frame, 0, HOVER.height - 1)[3], 0);
                assert_eq!(pixel(&frame, HOVER.width - 1, HOVER.height - 1)[3], 0);

                // Esquinas superiores: transparentes en `island`, opacas en `notch`.
                match style {
                    IslandStyle::Island => {
                        assert_eq!(pixel(&frame, 0, 0)[3], 0);
                        assert_eq!(pixel(&frame, HOVER.width - 1, 0)[3], 0);
                    }
                    IslandStyle::Notch => {
                        let expected_alpha =
                            effective_color(cfg.appearance.background, cfg.appearance.opacity).a;
                        assert_eq!(pixel(&frame, 0, 0)[3], expected_alpha);
                        assert_eq!(pixel(&frame, HOVER.width - 1, 0)[3], expected_alpha);
                    }
                }

                // El árbol respeta el padding y ningún nodo se sale de la máscara de la isla.
                let tree = build_expanded_tree_styled(
                    "21:07",
                    "sáb 14 nov",
                    HOVER,
                    CornerRadius::Auto,
                    style,
                );
                assert!(tree.padding.left >= 20 && tree.padding.right >= 20);
            }
        }

        // Verifica que un `text_color` personalizado en `config.ini` cambia los píxeles del texto/íconos.
        let mut cfg_red = Config::default();
        cfg_red.appearance.text_color = ConfigColor::rgb(255, 0, 0);
        let mut p_red = IslandPainter::from_config(cfg_red, None, None).expect("painter rojo");
        p_red.set_expanded(true);
        let frame_red = p_red.paint(HOVER, 1.0, sample_time()).expect("dibuja rojo");

        let frame_red_ref = &frame_red;
        let has_pure_red = (0..HOVER.height)
            .flat_map(|y| (0..HOVER.width).map(move |x| pixel(frame_red_ref, x, y)))
            .any(|[r, g, b, a]| r == 255 && g == 0 && b == 0 && a == 255);
        assert!(
            has_pure_red,
            "el árbol expandido usa el text_color configurado"
        );
    }

    #[test]
    fn all_weekday_month_time_and_date_characters_are_supported_by_embedded_font() {
        let tr = TextRenderer::new().expect("fuente embebida");
        for day_name in WEEKDAYS {
            for ch in day_name.chars() {
                assert!(tr.supports(ch), "falta {ch:?} de {day_name:?}");
            }
        }
        for month_name in MONTHS {
            for ch in month_name.chars() {
                assert!(tr.supports(ch), "falta {ch:?} de {month_name:?}");
            }
        }
        for weekday in 0..7u8 {
            for month in 1..=12u8 {
                let t = LocalTime {
                    year: 2026,
                    month,
                    day: 28,
                    weekday,
                    hour: 23,
                    minute: 59,
                    second: 0,
                    millisecond: 0,
                };
                for ch in format_time(&t).chars().chain(format_date(&t).chars()) {
                    assert!(tr.supports(ch), "falta {ch:?}");
                }
            }
        }
    }
}
