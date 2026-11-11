//! Configuración y temas de Atoll.
//!
//! Este crate define el formato propio de configuración de Atoll (estilo INI, sin
//! dependencias externas) junto con su parser y su serializador canónico.
//!
//! Reglas de diseño:
//! - Cero dependencias externas (sin `serde` ni `toml`) y cero I/O de archivos.
//! - El parser **nunca falla**: [`parse`] siempre devuelve una [`Config`] válida
//!   más una lista de [`Warning`] con el número de línea (empezando en 1).
//! - Un texto vacío produce [`Config::default`], que coincide con los valores por
//!   defecto actuales de la isla.
//! - Las claves o secciones desconocidas se ignoran y se informan como advertencia.
//! - Los valores inválidos o fuera de rango (incluyendo `hover_width < width` o
//!   `hover_height < height`) caen al valor por defecto (o al del tema elegido) y
//!   se informan como advertencia.
//! - En `[appearance]`, los valores explícitos válidos del archivo pisan siempre
//!   a los del preset `theme`, sin importar el orden en que aparezcan las claves.

use std::fmt;

/// Ancho lógico mínimo permitido para la isla (en píxeles a 96 DPI).
pub const MIN_WIDTH: u32 = 120;

/// Ancho lógico máximo permitido para la isla (en píxeles a 96 DPI).
pub const MAX_WIDTH: u32 = 1600;

/// Alto lógico mínimo permitido para la isla (en píxeles a 96 DPI).
pub const MIN_HEIGHT: u32 = 24;

/// Alto lógico máximo permitido para la isla (en píxeles a 96 DPI).
pub const MAX_HEIGHT: u32 = 400;

/// Margen superior lógico máximo permitido (en píxeles a 96 DPI).
pub const MAX_TOP_MARGIN: u32 = 200;

/// Opacidad máxima permitida (en porcentaje, de 0 a 100).
pub const MAX_OPACITY: u8 = 100;

/// Radio de esquina lógico máximo permitido cuando se fija en píxeles.
pub const MAX_CORNER_RADIUS: u32 = 200;

/// Ancho lógico por defecto de la isla en reposo.
pub const DEFAULT_WIDTH: u32 = 400;

/// Alto lógico por defecto de la isla en reposo.
pub const DEFAULT_HEIGHT: u32 = 40;

/// Ancho lógico por defecto de la isla al pasar el mouse.
pub const DEFAULT_HOVER_WIDTH: u32 = 480;

/// Alto lógico por defecto de la isla al pasar el mouse.
pub const DEFAULT_HOVER_HEIGHT: u32 = 64;

/// Margen superior lógico por defecto.
pub const DEFAULT_TOP_MARGIN: u32 = 8;

/// Color RGBA de 8 bits por canal en la configuración.
///
/// Se escribe en texto como `#RRGGBB` (con alfa implícito `255`) o `#RRGGBBAA`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    /// Canal rojo (0 a 255).
    pub r: u8,
    /// Canal verde (0 a 255).
    pub g: u8,
    /// Canal azul (0 a 255).
    pub b: u8,
    /// Canal alfa (0 = transparente, 255 = opaco).
    pub a: u8,
}

impl Color {
    /// Negro opaco (`#000000`).
    pub const BLACK: Self = Self::rgb(0, 0, 0);

    /// Blanco opaco (`#FFFFFF`).
    pub const WHITE: Self = Self::rgb(255, 255, 255);

    /// Crea un color opaco (`a = 255`) a partir de sus canales RGB.
    #[must_use]
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// Crea un color a partir de sus cuatro canales RGBA.
    #[must_use]
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Interpreta un color en formato `#RRGGBB` o `#RRGGBBAA`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let hex = text.trim().strip_prefix('#')?;
        match hex.len() {
            6 => {
                let r = parse_hex_byte(&hex[0..2])?;
                let g = parse_hex_byte(&hex[2..4])?;
                let b = parse_hex_byte(&hex[4..6])?;
                Some(Self::rgb(r, g, b))
            }
            8 => {
                let r = parse_hex_byte(&hex[0..2])?;
                let g = parse_hex_byte(&hex[2..4])?;
                let b = parse_hex_byte(&hex[4..6])?;
                let a = parse_hex_byte(&hex[6..8])?;
                Some(Self::rgba(r, g, b, a))
            }
            _ => None,
        }
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.a == 255 {
            write!(f, "#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
        } else {
            write!(
                f,
                "#{:02X}{:02X}{:02X}{:02X}",
                self.r, self.g, self.b, self.a
            )
        }
    }
}

/// Tema visual predefinido para la isla.
///
/// Define los valores base de `background`, `text_color`, `opacity` y
/// `corner_radius`. Cualquier valor explícito válido en `[appearance]` pisa
/// al del tema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Theme {
    /// Negro puro (`#000000`) con texto blanco (`#FFFFFF`) y opacidad 100 %.
    #[default]
    Black,
    /// Gris grafito oscuro (`#1C1C1E`) con texto claro (`#F5F5F7`) y opacidad 95 %.
    Graphite,
    /// Blanco suave (`#F5F5F7`) con texto oscuro (`#111111`) y opacidad 95 %.
    White,
}

impl Theme {
    /// Interpreta el nombre de un tema (`black`, `graphite`, `white`).
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        if text.trim().eq_ignore_ascii_case("black") {
            Some(Self::Black)
        } else if text.trim().eq_ignore_ascii_case("graphite") {
            Some(Self::Graphite)
        } else if text.trim().eq_ignore_ascii_case("white") {
            Some(Self::White)
        } else {
            None
        }
    }

    /// Nombre canónico del tema en el archivo de configuración.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Black => "black",
            Self::Graphite => "graphite",
            Self::White => "white",
        }
    }

    /// Devuelve la sección `[appearance]` completa que corresponde a este preset.
    #[must_use]
    pub const fn appearance(self) -> AppearanceSection {
        match self {
            Self::Black => AppearanceSection {
                theme: Self::Black,
                background: Color::BLACK,
                text_color: Color::WHITE,
                opacity: 100,
                corner_radius: CornerRadius::Auto,
            },
            Self::Graphite => AppearanceSection {
                theme: Self::Graphite,
                background: Color::rgb(0x1C, 0x1C, 0x1E),
                text_color: Color::rgb(0xF5, 0xF5, 0xF7),
                opacity: 95,
                corner_radius: CornerRadius::Auto,
            },
            Self::White => AppearanceSection {
                theme: Self::White,
                background: Color::rgb(0xF5, 0xF5, 0xF7),
                text_color: Color::rgb(0x11, 0x11, 0x11),
                opacity: 95,
                corner_radius: CornerRadius::Auto,
            },
        }
    }
}

impl fmt::Display for Theme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Radio de las esquinas de la isla.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CornerRadius {
    /// Forma de píldora automática (`min(ancho, alto) / 2`).
    #[default]
    Auto,
    /// Radio fijo en píxeles lógicos (de `0` a [`MAX_CORNER_RADIUS`]).
    Px(u32),
}

impl CornerRadius {
    /// Interpreta `"auto"` o un entero en `0..=MAX_CORNER_RADIUS`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let trimmed = text.trim();
        if trimmed.eq_ignore_ascii_case("auto") {
            return Some(Self::Auto);
        }
        let px = trimmed.parse::<u32>().ok()?;
        if px <= MAX_CORNER_RADIUS {
            Some(Self::Px(px))
        } else {
            None
        }
    }
}

impl fmt::Display for CornerRadius {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Auto => f.write_str("auto"),
            Self::Px(px) => write!(f, "{px}"),
        }
    }
}

/// Interpreta un booleano del formato de configuración (`true` o `false`).
#[must_use]
pub fn parse_bool(text: &str) -> Option<bool> {
    let trimmed = text.trim();
    if trimmed.eq_ignore_ascii_case("true") {
        Some(true)
    } else if trimmed.eq_ignore_ascii_case("false") {
        Some(false)
    } else {
        None
    }
}

/// Sección `[island]`: medidas lógicas de la isla a 96 DPI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IslandSection {
    /// Ancho lógico en reposo (`MIN_WIDTH..=MAX_WIDTH`).
    pub width: u32,
    /// Alto lógico en reposo (`MIN_HEIGHT..=MAX_HEIGHT`).
    pub height: u32,
    /// Ancho lógico al pasar el mouse (`MIN_WIDTH..=MAX_WIDTH`, siempre `>= width`).
    pub hover_width: u32,
    /// Alto lógico al pasar el mouse (`MIN_HEIGHT..=MAX_HEIGHT`, siempre `>= height`).
    pub hover_height: u32,
    /// Margen superior lógico respecto del borde de la pantalla (`0..=MAX_TOP_MARGIN`).
    pub top_margin: u32,
}

impl Default for IslandSection {
    fn default() -> Self {
        Self {
            width: DEFAULT_WIDTH,
            height: DEFAULT_HEIGHT,
            hover_width: DEFAULT_HOVER_WIDTH,
            hover_height: DEFAULT_HOVER_HEIGHT,
            top_margin: DEFAULT_TOP_MARGIN,
        }
    }
}

/// Sección `[appearance]`: tema, colores, opacidad y radio de borde.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppearanceSection {
    /// Tema base seleccionado.
    pub theme: Theme,
    /// Color de fondo de la isla.
    pub background: Color,
    /// Color principal del texto.
    pub text_color: Color,
    /// Opacidad general de la isla, de `0` a `100`.
    pub opacity: u8,
    /// Radio de las esquinas (`auto` o píxeles lógicos).
    pub corner_radius: CornerRadius,
}

impl Default for AppearanceSection {
    fn default() -> Self {
        Theme::Black.appearance()
    }
}

/// Sección `[behavior]`: reservada para el comportamiento configurable (ítem 12).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BehaviorSection {}

/// Configuración completa de Atoll.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Config {
    /// Medidas y posición de la isla (`[island]`).
    pub island: IslandSection,
    /// Aspecto visual y tema (`[appearance]`).
    pub appearance: AppearanceSection,
    /// Comportamiento de la isla (`[behavior]`).
    pub behavior: BehaviorSection,
}

impl Config {
    /// Analiza un texto de configuración y devuelve la [`Config`] resultante junto con
    /// las advertencias encontradas. Nunca falla.
    #[must_use]
    pub fn parse(text: &str) -> (Self, Vec<Warning>) {
        parse(text)
    }

    /// Serializa la configuración en el formato canónico con comentarios explicativos.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut out = String::with_capacity(768);
        out.push_str("# Configuración de Atoll\n");
        out.push_str(
            "# Todas las medidas están en píxeles lógicos a 96 DPI (100 % de escala).\n\n",
        );

        out.push_str("[island]\n");
        out.push_str("# Tamaño de la isla en reposo (ancho: 120..1600, alto: 24..400).\n");
        out.push_str(&format!("width = {}\n", self.island.width));
        out.push_str(&format!("height = {}\n", self.island.height));
        out.push_str(
            "# Tamaño de la isla al pasar el mouse (ancho: 120..1600 >= width, alto: 24..400 >= height).\n",
        );
        out.push_str(&format!("hover_width = {}\n", self.island.hover_width));
        out.push_str(&format!("hover_height = {}\n", self.island.hover_height));
        out.push_str("# Separación con el borde superior de la pantalla (0..200).\n");
        out.push_str(&format!("top_margin = {}\n\n", self.island.top_margin));

        out.push_str("[appearance]\n");
        out.push_str("# Tema base: black, graphite o white. Los campos de abajo pisan al tema.\n");
        out.push_str(&format!("theme = {}\n", self.appearance.theme));
        out.push_str("# Color de fondo y de texto (#RRGGBB o #RRGGBBAA).\n");
        out.push_str(&format!("background = {}\n", self.appearance.background));
        out.push_str(&format!("text_color = {}\n", self.appearance.text_color));
        out.push_str("# Opacidad general de la isla (0 a 100).\n");
        out.push_str(&format!("opacity = {}\n", self.appearance.opacity));
        out.push_str("# Radio de esquina: \"auto\" (píldora) o píxeles lógicos (0..200).\n");
        out.push_str(&format!(
            "corner_radius = {}\n\n",
            self.appearance.corner_radius
        ));

        out.push_str("[behavior]\n");
        out.push_str("# Reservado para opciones de comportamiento.\n");
        out
    }
}

/// Advertencia producida al leer una línea del archivo de configuración.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Warning {
    /// Número de línea en el texto (empezando en 1).
    pub line: usize,
    /// Motivo de la advertencia.
    pub kind: WarningKind,
}

/// Tipo de advertencia detectada por el parser de configuración.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WarningKind {
    /// La línea no tiene formato válido de `[sección]` ni de `clave = valor`.
    InvalidSyntax,
    /// Apareció una `clave = valor` antes de abrir cualquier `[sección]`.
    KeyOutsideSection {
        /// Nombre de la clave encontrada fuera de sección.
        key: String,
    },
    /// Sección desconocida; sus claves se ignoran.
    UnknownSection {
        /// Nombre de la sección desconocida.
        section: String,
    },
    /// Clave desconocida dentro de una sección conocida.
    UnknownKey {
        /// Sección donde apareció la clave.
        section: String,
        /// Nombre de la clave desconocida.
        key: String,
    },
    /// El valor de una clave conocida es inválido o está fuera del rango permitido.
    InvalidValue {
        /// Sección de la clave.
        section: String,
        /// Nombre de la clave.
        key: String,
        /// Valor rechazado tal como vino en el archivo.
        value: String,
    },
}

impl fmt::Display for WarningKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSyntax => write!(f, "sintaxis inválida (se esperaba 'clave = valor')"),
            Self::KeyOutsideSection { key } => {
                write!(f, "la clave '{key}' está fuera de cualquier sección")
            }
            Self::UnknownSection { section } => write!(f, "sección desconocida '[{section}]'"),
            Self::UnknownKey { section, key } => {
                write!(f, "clave desconocida '{key}' en [{section}]")
            }
            Self::InvalidValue {
                section,
                key,
                value,
            } => write!(
                f,
                "valor inválido '{value}' para '{key}' en [{section}]; se usa el valor por defecto"
            ),
        }
    }
}

impl fmt::Display for Warning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "línea {}: {}", self.line, self.kind)
    }
}

/// Sección activa mientras se recorre el archivo línea por línea.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActiveSection {
    None,
    Island,
    Appearance,
    Behavior,
    Unknown,
}

/// Entrada explícita leída para un tamaño de `[island]`.
#[derive(Debug, Clone)]
struct SizeEntry {
    value: u32,
    line: usize,
    raw: String,
}

/// Valores leídos en `[island]`, para validar además que `hover >= reposo`.
#[derive(Debug, Default)]
struct IslandOverrides {
    width: Option<SizeEntry>,
    height: Option<SizeEntry>,
    hover_width: Option<SizeEntry>,
    hover_height: Option<SizeEntry>,
    top_margin: Option<u32>,
}

impl IslandOverrides {
    /// Resuelve `[island]` verificando que `hover_width >= width` y `hover_height >= height`.
    fn resolve(self, warnings: &mut Vec<Warning>) -> IslandSection {
        let mut section = IslandSection::default();
        if let Some(top_margin) = self.top_margin {
            section.top_margin = top_margin;
        }

        let width = self.width.as_ref().map_or(DEFAULT_WIDTH, |e| e.value);
        let hover_width = self
            .hover_width
            .as_ref()
            .map_or(DEFAULT_HOVER_WIDTH, |e| e.value);
        if hover_width >= width {
            section.width = width;
            section.hover_width = hover_width;
        } else if let Some(entry) = self.hover_width {
            push_invalid(warnings, entry.line, "island", "hover_width", &entry.raw);
        } else if let Some(entry) = self.width {
            push_invalid(warnings, entry.line, "island", "width", &entry.raw);
        }

        let height = self.height.as_ref().map_or(DEFAULT_HEIGHT, |e| e.value);
        let hover_height = self
            .hover_height
            .as_ref()
            .map_or(DEFAULT_HOVER_HEIGHT, |e| e.value);
        if hover_height >= height {
            section.height = height;
            section.hover_height = hover_height;
        } else if let Some(entry) = self.hover_height {
            push_invalid(warnings, entry.line, "island", "hover_height", &entry.raw);
        } else if let Some(entry) = self.height {
            push_invalid(warnings, entry.line, "island", "height", &entry.raw);
        }

        section
    }
}

/// Valores explícitos leídos en `[appearance]`, para aplicarlos por encima del `Theme`.
#[derive(Debug, Default)]
struct AppearanceOverrides {
    theme: Option<Theme>,
    background: Option<Color>,
    text_color: Option<Color>,
    opacity: Option<u8>,
    corner_radius: Option<CornerRadius>,
}

impl AppearanceOverrides {
    /// Combina el preset del tema elegido con los campos explícitos válidos.
    fn resolve(self) -> AppearanceSection {
        let theme = self.theme.unwrap_or_default();
        let mut section = theme.appearance();
        if let Some(background) = self.background {
            section.background = background;
        }
        if let Some(text_color) = self.text_color {
            section.text_color = text_color;
        }
        if let Some(opacity) = self.opacity {
            section.opacity = opacity;
        }
        if let Some(corner_radius) = self.corner_radius {
            section.corner_radius = corner_radius;
        }
        section
    }
}

/// Analiza un texto de configuración en el formato de Atoll.
///
/// Devuelve siempre una [`Config`] válida y la lista de [`Warning`] en orden de
/// aparición. Un texto vacío produce [`Config::default`] sin advertencias.
#[must_use]
pub fn parse(text: &str) -> (Config, Vec<Warning>) {
    let mut config = Config::default();
    let mut island_overrides = IslandOverrides::default();
    let mut appearance_overrides = AppearanceOverrides::default();
    let mut warnings = Vec::new();
    let mut section = ActiveSection::None;

    for (index, raw_line) in text.lines().enumerate() {
        let line_no = index + 1;
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line.starts_with('[') {
            let without_comment = strip_inline_comment(line);
            let Some(name) = without_comment
                .strip_prefix('[')
                .and_then(|s| s.strip_suffix(']'))
                .map(str::trim)
                .filter(|s| !s.is_empty())
            else {
                warnings.push(Warning {
                    line: line_no,
                    kind: WarningKind::InvalidSyntax,
                });
                continue;
            };

            if name.eq_ignore_ascii_case("island") {
                section = ActiveSection::Island;
            } else if name.eq_ignore_ascii_case("appearance") {
                section = ActiveSection::Appearance;
            } else if name.eq_ignore_ascii_case("behavior") {
                section = ActiveSection::Behavior;
            } else {
                section = ActiveSection::Unknown;
                warnings.push(Warning {
                    line: line_no,
                    kind: WarningKind::UnknownSection {
                        section: name.to_owned(),
                    },
                });
            }
            continue;
        }

        let Some((raw_key, raw_value)) = line.split_once('=') else {
            warnings.push(Warning {
                line: line_no,
                kind: WarningKind::InvalidSyntax,
            });
            continue;
        };

        let key = raw_key.trim();
        if key.is_empty() {
            warnings.push(Warning {
                line: line_no,
                kind: WarningKind::InvalidSyntax,
            });
            continue;
        }
        let value = strip_inline_comment(raw_value.trim());

        match section {
            ActiveSection::None => {
                warnings.push(Warning {
                    line: line_no,
                    kind: WarningKind::KeyOutsideSection {
                        key: key.to_owned(),
                    },
                });
            }
            ActiveSection::Unknown => {}
            ActiveSection::Island => {
                parse_island_key(&mut island_overrides, key, value, line_no, &mut warnings);
            }
            ActiveSection::Appearance => {
                parse_appearance_key(
                    &mut appearance_overrides,
                    key,
                    value,
                    line_no,
                    &mut warnings,
                );
            }
            ActiveSection::Behavior => {
                warnings.push(Warning {
                    line: line_no,
                    kind: WarningKind::UnknownKey {
                        section: "behavior".to_owned(),
                        key: key.to_owned(),
                    },
                });
            }
        }
    }

    config.island = island_overrides.resolve(&mut warnings);
    config.appearance = appearance_overrides.resolve();
    warnings.sort_by_key(|w| w.line);
    (config, warnings)
}

/// Procesa un par `clave = valor` dentro de `[island]`.
fn parse_island_key(
    island: &mut IslandOverrides,
    key: &str,
    value: &str,
    line: usize,
    warnings: &mut Vec<Warning>,
) {
    match key {
        "width" => {
            if let Some(v) = parse_u32_range(value, MIN_WIDTH, MAX_WIDTH) {
                island.width = Some(SizeEntry {
                    value: v,
                    line,
                    raw: value.to_owned(),
                });
            } else {
                island.width = None;
                push_invalid(warnings, line, "island", key, value);
            }
        }
        "height" => {
            if let Some(v) = parse_u32_range(value, MIN_HEIGHT, MAX_HEIGHT) {
                island.height = Some(SizeEntry {
                    value: v,
                    line,
                    raw: value.to_owned(),
                });
            } else {
                island.height = None;
                push_invalid(warnings, line, "island", key, value);
            }
        }
        "hover_width" => {
            if let Some(v) = parse_u32_range(value, MIN_WIDTH, MAX_WIDTH) {
                island.hover_width = Some(SizeEntry {
                    value: v,
                    line,
                    raw: value.to_owned(),
                });
            } else {
                island.hover_width = None;
                push_invalid(warnings, line, "island", key, value);
            }
        }
        "hover_height" => {
            if let Some(v) = parse_u32_range(value, MIN_HEIGHT, MAX_HEIGHT) {
                island.hover_height = Some(SizeEntry {
                    value: v,
                    line,
                    raw: value.to_owned(),
                });
            } else {
                island.hover_height = None;
                push_invalid(warnings, line, "island", key, value);
            }
        }
        "top_margin" => {
            if let Some(v) = parse_u32_range(value, 0, MAX_TOP_MARGIN) {
                island.top_margin = Some(v);
            } else {
                island.top_margin = None;
                push_invalid(warnings, line, "island", key, value);
            }
        }
        _ => {
            warnings.push(Warning {
                line,
                kind: WarningKind::UnknownKey {
                    section: "island".to_owned(),
                    key: key.to_owned(),
                },
            });
        }
    }
}

/// Procesa un par `clave = valor` dentro de `[appearance]`.
fn parse_appearance_key(
    overrides: &mut AppearanceOverrides,
    key: &str,
    value: &str,
    line: usize,
    warnings: &mut Vec<Warning>,
) {
    match key {
        "theme" => {
            if let Some(theme) = Theme::parse(value) {
                overrides.theme = Some(theme);
            } else {
                overrides.theme = None;
                push_invalid(warnings, line, "appearance", key, value);
            }
        }
        "background" => {
            if let Some(color) = Color::parse(value) {
                overrides.background = Some(color);
            } else {
                overrides.background = None;
                push_invalid(warnings, line, "appearance", key, value);
            }
        }
        "text_color" => {
            if let Some(color) = Color::parse(value) {
                overrides.text_color = Some(color);
            } else {
                overrides.text_color = None;
                push_invalid(warnings, line, "appearance", key, value);
            }
        }
        "opacity" => {
            if let Some(opacity) = parse_u8_range(value, 0, MAX_OPACITY) {
                overrides.opacity = Some(opacity);
            } else {
                overrides.opacity = None;
                push_invalid(warnings, line, "appearance", key, value);
            }
        }
        "corner_radius" => {
            if let Some(radius) = CornerRadius::parse(value) {
                overrides.corner_radius = Some(radius);
            } else {
                overrides.corner_radius = None;
                push_invalid(warnings, line, "appearance", key, value);
            }
        }
        _ => {
            warnings.push(Warning {
                line,
                kind: WarningKind::UnknownKey {
                    section: "appearance".to_owned(),
                    key: key.to_owned(),
                },
            });
        }
    }
}

/// Agrega una advertencia de valor inválido.
fn push_invalid(warnings: &mut Vec<Warning>, line: usize, section: &str, key: &str, value: &str) {
    warnings.push(Warning {
        line,
        kind: WarningKind::InvalidValue {
            section: section.to_owned(),
            key: key.to_owned(),
            value: value.to_owned(),
        },
    });
}

/// Quita un comentario final (` # ...`) sin romper colores que empiezan con `#`.
fn strip_inline_comment(text: &str) -> &str {
    let bytes = text.as_bytes();
    for (i, &byte) in bytes.iter().enumerate().skip(1) {
        if byte == b'#' && bytes[i - 1].is_ascii_whitespace() {
            return text[..i].trim_end();
        }
    }
    text.trim_end()
}

/// Parsea dos dígitos hexadecimales como un `u8`.
fn parse_hex_byte(pair: &str) -> Option<u8> {
    u8::from_str_radix(pair, 16).ok()
}

/// Parsea un `u32` dentro del rango inclusivo `min..=max`.
fn parse_u32_range(text: &str, min: u32, max: u32) -> Option<u32> {
    let value = text.trim().parse::<u32>().ok()?;
    if (min..=max).contains(&value) {
        Some(value)
    } else {
        None
    }
}

/// Parsea un `u8` dentro del rango inclusivo `min..=max`.
fn parse_u8_range(text: &str, min: u8, max: u8) -> Option<u8> {
    let value = text.trim().parse::<u8>().ok()?;
    if (min..=max).contains(&value) {
        Some(value)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{
        parse, parse_bool, Color, Config, CornerRadius, Theme, Warning, WarningKind,
        DEFAULT_HEIGHT, DEFAULT_HOVER_HEIGHT, DEFAULT_HOVER_WIDTH, DEFAULT_TOP_MARGIN,
        DEFAULT_WIDTH,
    };

    #[test]
    fn empty_or_comment_only_text_returns_default_without_warnings() {
        let (cfg, warnings) = parse("");
        assert_eq!(cfg, Config::default());
        assert!(warnings.is_empty());

        let only_comments = "
            # Archivo de prueba
            # Solo comentarios y líneas en blanco

        ";
        let (cfg2, warnings2) = parse(only_comments);
        assert_eq!(cfg2, Config::default());
        assert!(warnings2.is_empty());
    }

    #[test]
    fn default_values_match_current_island_constants() {
        let cfg = Config::default();
        assert_eq!(cfg.island.width, 400);
        assert_eq!(cfg.island.height, 40);
        assert_eq!(cfg.island.hover_width, 480);
        assert_eq!(cfg.island.hover_height, 64);
        assert_eq!(cfg.island.top_margin, 8);
        assert_eq!(cfg.appearance.theme, Theme::Black);
        assert_eq!(cfg.appearance.background, Color::BLACK);
        assert_eq!(cfg.appearance.text_color, Color::WHITE);
        assert_eq!(cfg.appearance.opacity, 100);
        assert_eq!(cfg.appearance.corner_radius, CornerRadius::Auto);
    }

    #[test]
    fn valid_values_are_parsed_across_all_sections() {
        let input = "
[island]
width = 420
height = 44
hover_width = 520
hover_height = 72
top_margin = 12 # margen superior

[appearance]
theme = graphite
background = #102030
text_color = #AABBCCDD
opacity = 85
corner_radius = 18

[behavior]
";
        let (cfg, warnings) = parse(input);
        assert!(warnings.is_empty(), "sin advertencias: {warnings:?}");
        assert_eq!(cfg.island.width, 420);
        assert_eq!(cfg.island.height, 44);
        assert_eq!(cfg.island.hover_width, 520);
        assert_eq!(cfg.island.hover_height, 72);
        assert_eq!(cfg.island.top_margin, 12);
        assert_eq!(cfg.appearance.theme, Theme::Graphite);
        assert_eq!(cfg.appearance.background, Color::rgb(0x10, 0x20, 0x30));
        assert_eq!(
            cfg.appearance.text_color,
            Color::rgba(0xAA, 0xBB, 0xCC, 0xDD)
        );
        assert_eq!(cfg.appearance.opacity, 85);
        assert_eq!(cfg.appearance.corner_radius, CornerRadius::Px(18));
    }

    #[test]
    fn hover_size_smaller_than_rest_size_falls_back_to_defaults_with_warning() {
        let input = "\
[island]
width = 500
hover_width = 440
height = 80
hover_height = 60
";
        let (cfg, warnings) = parse(input);
        assert_eq!(cfg.island.width, DEFAULT_WIDTH);
        assert_eq!(cfg.island.hover_width, DEFAULT_HOVER_WIDTH);
        assert_eq!(cfg.island.height, DEFAULT_HEIGHT);
        assert_eq!(cfg.island.hover_height, DEFAULT_HOVER_HEIGHT);
        assert_eq!(
            warnings,
            vec![
                Warning {
                    line: 3,
                    kind: WarningKind::InvalidValue {
                        section: "island".to_owned(),
                        key: "hover_width".to_owned(),
                        value: "440".to_owned(),
                    },
                },
                Warning {
                    line: 5,
                    kind: WarningKind::InvalidValue {
                        section: "island".to_owned(),
                        key: "hover_height".to_owned(),
                        value: "60".to_owned(),
                    },
                },
            ]
        );

        // Si solo se indica `width = 600` (mayor que el `hover_width` por defecto 480),
        // también se restaura el par por defecto y se informa en la línea de `width`.
        let only_width = "[island]\nwidth = 600\n";
        let (cfg2, warnings2) = parse(only_width);
        assert_eq!(cfg2.island.width, DEFAULT_WIDTH);
        assert_eq!(cfg2.island.hover_width, DEFAULT_HOVER_WIDTH);
        assert_eq!(
            warnings2,
            vec![Warning {
                line: 2,
                kind: WarningKind::InvalidValue {
                    section: "island".to_owned(),
                    key: "width".to_owned(),
                    value: "600".to_owned(),
                },
            }]
        );
    }

    #[test]
    fn colors_support_both_rrggbb_and_rrggbbaa_case_insensitively() {
        assert_eq!(Color::parse("#000000"), Some(Color::rgb(0, 0, 0)));
        assert_eq!(Color::parse("#ff8040"), Some(Color::rgb(255, 128, 64)));
        assert_eq!(
            Color::parse("#11223344"),
            Some(Color::rgba(0x11, 0x22, 0x33, 0x44))
        );
        assert_eq!(Color::rgb(0x0A, 0xBC, 0xDE).to_string(), "#0ABCDE");
        assert_eq!(Color::rgba(0x0A, 0xBC, 0xDE, 0x80).to_string(), "#0ABCDE80");

        for invalid in ["000000", "#123", "#12345", "#1234567", "#GGGGGG", ""] {
            assert_eq!(Color::parse(invalid), None, "debe rechazar {invalid:?}");
        }
    }

    #[test]
    fn booleans_and_enums_parse_cleanly() {
        assert_eq!(parse_bool("true"), Some(true));
        assert_eq!(parse_bool("TRUE"), Some(true));
        assert_eq!(parse_bool("false"), Some(false));
        assert_eq!(parse_bool("False"), Some(false));
        assert_eq!(parse_bool("1"), None);
        assert_eq!(parse_bool("si"), None);

        assert_eq!(CornerRadius::parse("auto"), Some(CornerRadius::Auto));
        assert_eq!(CornerRadius::parse("AUTO"), Some(CornerRadius::Auto));
        assert_eq!(CornerRadius::parse("0"), Some(CornerRadius::Px(0)));
        assert_eq!(CornerRadius::parse("24"), Some(CornerRadius::Px(24)));
        assert_eq!(CornerRadius::parse("201"), None);
        assert_eq!(CornerRadius::parse("-5"), None);
    }

    #[test]
    fn preset_sets_appearance_and_explicit_fields_override_it_in_any_order() {
        let (white_only, warnings) = parse("[appearance]\ntheme = white\n");
        assert!(warnings.is_empty());
        assert_eq!(white_only.appearance, Theme::White.appearance());

        let after = "
[appearance]
theme = white
background = #223344
opacity = 80
";
        let (cfg_after, warnings_after) = parse(after);
        assert!(warnings_after.is_empty());
        assert_eq!(cfg_after.appearance.theme, Theme::White);
        assert_eq!(
            cfg_after.appearance.background,
            Color::rgb(0x22, 0x33, 0x44)
        );
        assert_eq!(
            cfg_after.appearance.text_color,
            Theme::White.appearance().text_color
        );
        assert_eq!(cfg_after.appearance.opacity, 80);

        let before = "
[appearance]
background = #223344
opacity = 80
theme = white
";
        let (cfg_before, warnings_before) = parse(before);
        assert!(warnings_before.is_empty());
        assert_eq!(cfg_before.appearance, cfg_after.appearance);
    }

    #[test]
    fn invalid_and_out_of_range_values_fall_back_to_defaults_with_line_numbers() {
        let input = "\
[island]
width = 50
height = 9999
hover_width = abc
hover_height = -10
top_margin = 500

[appearance]
theme = neon
background = #12345
text_color = azul
opacity = 150
corner_radius = 999
";
        let (cfg, warnings) = parse(input);
        assert_eq!(cfg.island.width, DEFAULT_WIDTH);
        assert_eq!(cfg.island.height, DEFAULT_HEIGHT);
        assert_eq!(cfg.island.hover_width, DEFAULT_HOVER_WIDTH);
        assert_eq!(cfg.island.hover_height, DEFAULT_HOVER_HEIGHT);
        assert_eq!(cfg.island.top_margin, DEFAULT_TOP_MARGIN);
        assert_eq!(cfg.appearance, Theme::Black.appearance());

        let expected = vec![
            Warning {
                line: 2,
                kind: WarningKind::InvalidValue {
                    section: "island".to_owned(),
                    key: "width".to_owned(),
                    value: "50".to_owned(),
                },
            },
            Warning {
                line: 3,
                kind: WarningKind::InvalidValue {
                    section: "island".to_owned(),
                    key: "height".to_owned(),
                    value: "9999".to_owned(),
                },
            },
            Warning {
                line: 4,
                kind: WarningKind::InvalidValue {
                    section: "island".to_owned(),
                    key: "hover_width".to_owned(),
                    value: "abc".to_owned(),
                },
            },
            Warning {
                line: 5,
                kind: WarningKind::InvalidValue {
                    section: "island".to_owned(),
                    key: "hover_height".to_owned(),
                    value: "-10".to_owned(),
                },
            },
            Warning {
                line: 6,
                kind: WarningKind::InvalidValue {
                    section: "island".to_owned(),
                    key: "top_margin".to_owned(),
                    value: "500".to_owned(),
                },
            },
            Warning {
                line: 9,
                kind: WarningKind::InvalidValue {
                    section: "appearance".to_owned(),
                    key: "theme".to_owned(),
                    value: "neon".to_owned(),
                },
            },
            Warning {
                line: 10,
                kind: WarningKind::InvalidValue {
                    section: "appearance".to_owned(),
                    key: "background".to_owned(),
                    value: "#12345".to_owned(),
                },
            },
            Warning {
                line: 11,
                kind: WarningKind::InvalidValue {
                    section: "appearance".to_owned(),
                    key: "text_color".to_owned(),
                    value: "azul".to_owned(),
                },
            },
            Warning {
                line: 12,
                kind: WarningKind::InvalidValue {
                    section: "appearance".to_owned(),
                    key: "opacity".to_owned(),
                    value: "150".to_owned(),
                },
            },
            Warning {
                line: 13,
                kind: WarningKind::InvalidValue {
                    section: "appearance".to_owned(),
                    key: "corner_radius".to_owned(),
                    value: "999".to_owned(),
                },
            },
        ];
        assert_eq!(warnings, expected);
    }

    #[test]
    fn unknown_keys_sections_and_bad_syntax_report_exact_line_numbers() {
        let input = "\
suelta = 1
[island]
width = 450
futuro = true
sin_igual
[extra]
algo = 2
[behavior]
 futura_opcion = 10
";
        let (cfg, warnings) = parse(input);
        assert_eq!(cfg.island.width, 450);

        assert_eq!(
            warnings,
            vec![
                Warning {
                    line: 1,
                    kind: WarningKind::KeyOutsideSection {
                        key: "suelta".to_owned(),
                    },
                },
                Warning {
                    line: 4,
                    kind: WarningKind::UnknownKey {
                        section: "island".to_owned(),
                        key: "futuro".to_owned(),
                    },
                },
                Warning {
                    line: 5,
                    kind: WarningKind::InvalidSyntax,
                },
                Warning {
                    line: 6,
                    kind: WarningKind::UnknownSection {
                        section: "extra".to_owned(),
                    },
                },
                Warning {
                    line: 9,
                    kind: WarningKind::UnknownKey {
                        section: "behavior".to_owned(),
                        key: "futura_opcion".to_owned(),
                    },
                },
            ]
        );
        assert!(warnings[0].to_string().contains("línea 1"));
    }

    #[test]
    fn roundtrip_parse_to_text_parse_is_identical() {
        let original = Config::default();
        let text = original.to_text();
        let (reparsed, warnings) = Config::parse(&text);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(reparsed, original);
        assert_eq!(reparsed.to_text(), text);

        let custom_input = "
[island]
width = 500
height = 48
hover_width = 620
hover_height = 80
top_margin = 16

[appearance]
theme = graphite
background = #11223344
text_color = #FAFAFA
opacity = 90
corner_radius = 14
";
        let (custom, w1) = parse(custom_input);
        assert!(w1.is_empty());
        let serialized = custom.to_text();
        let (again, w2) = parse(&serialized);
        assert!(w2.is_empty());
        assert_eq!(custom, again);
    }
}
