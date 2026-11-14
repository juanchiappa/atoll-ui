//! Mini toolkit declarativo de widgets de Atoll (`atoll-ui`).
//!
//! Este crate implementa las capas puras de interfaz de usuario de Atoll, sin
//! dependencias externas ni código específico de ningún sistema operativo:
//!
//! 1. **Árbol de widgets ([`Node`], [`WidgetKind`]):** estructuras de datos planas
//!    (`Clone + Debug + PartialEq + Eq`), sin closures ni referencias, listas para
//!    serializarse por IPC entre los plugins y el Core.
//! 2. **Validación ([`validate_tree`]):** verifica límites duros (`MAX_TREE_DEPTH = 32`,
//!    `MAX_TREE_NODES = 1024`), identificadores duplicados y rangos de medidas sin
//!    recursión descontrolada ni pánicos.
//! 3. **Motor de layout ([`compute_layout`]):** algoritmo determinista tipo flex
//!    (`Auto`, `Fixed`, `Fill`, `Edges`, `MainAlign`, `CrossAlign`) con aritmética
//!    entera saturada/chequeada y recorte definido dentro del rectángulo del padre.
//! 4. **Hit-testing y estado interactivo ([`hit_test`], [`UiState`]):** máquina de
//!    estados pura y determinista que procesa entradas del puntero ([`PointerInput`]),
//!    emite eventos de alto nivel ([`UiEvent`]) sin mutar el árbol y calcula con
//!    exactitud cuándo hace falta redibujar (`needs_redraw`).

use std::fmt;

/// Profundidad máxima permitida para un árbol de widgets (la raíz tiene profundidad 1).
pub const MAX_TREE_DEPTH: usize = 32;

/// Cantidad máxima de nodos permitida en un árbol de widgets.
pub const MAX_TREE_NODES: usize = 1024;

/// Valor máximo permitido para cualquier medida lógica en píxeles a 96 DPI (`4096` px).
pub const MAX_LOGICAL_PX: u16 = 4096;

/// Escala por defecto expresada en milésimas (`1000` = 100 % = 1,0x).
pub const DEFAULT_SCALE_MILLI: u32 = 1000;

/// Escala mínima permitida en milésimas (`250` = 25 % = 0,25x).
pub const MIN_SCALE_MILLI: u32 = 250;

/// Escala máxima permitida en milésimas (`8000` = 800 % = 8,0x).
pub const MAX_SCALE_MILLI: u32 = 8000;

/// Altura mínima lógica de un [`WidgetKind::Button`] con `Length::Auto`, en píxeles a 96 DPI.
pub const BUTTON_MIN_HEIGHT_PX: u16 = 28;

/// Relleno horizontal lógico interno por lado de un [`WidgetKind::Button`], en píxeles a 96 DPI.
pub const BUTTON_PAD_X_PX: u16 = 12;

/// Relleno vertical lógico interno por lado de un [`WidgetKind::Button`], en píxeles a 96 DPI.
pub const BUTTON_PAD_Y_PX: u16 = 6;

/// Tamaño lógico del ícono dentro de un [`WidgetKind::Button`], en píxeles a 96 DPI.
pub const BUTTON_ICON_PX: u16 = 16;

/// Tamaño lógico del texto dentro de un [`WidgetKind::Button`], en píxeles a 96 DPI.
pub const BUTTON_TEXT_PX: u16 = 14;

/// Separación lógica entre ícono y texto en un [`WidgetKind::Button`], en píxeles a 96 DPI.
pub const BUTTON_GAP_PX: u16 = 6;

/// Ancho lógico intrínseco de un [`WidgetKind::Toggle`], en píxeles a 96 DPI.
pub const TOGGLE_WIDTH_PX: u16 = 36;

/// Alto lógico intrínseco de un [`WidgetKind::Toggle`], en píxeles a 96 DPI.
pub const TOGGLE_HEIGHT_PX: u16 = 20;

/// Ancho lógico mínimo intrínseco de un [`WidgetKind::Slider`], en píxeles a 96 DPI.
pub const SLIDER_MIN_WIDTH_PX: u16 = 80;

/// Alto lógico intrínseco de un [`WidgetKind::Slider`], en píxeles a 96 DPI.
pub const SLIDER_HEIGHT_PX: u16 = 20;

/// Ancho lógico mínimo intrínseco de un [`WidgetKind::Progress`], en píxeles a 96 DPI.
pub const PROGRESS_MIN_WIDTH_PX: u16 = 60;

/// Alto lógico intrínseco de un [`WidgetKind::Progress`], en píxeles a 96 DPI.
pub const PROGRESS_HEIGHT_PX: u16 = 6;

/// Dimensiones enteras en píxeles (`width` × `height`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Size {
    /// Ancho en píxeles.
    pub width: u32,
    /// Alto en píxeles.
    pub height: u32,
}

impl Size {
    /// Tamaño nulo (`0 × 0`).
    pub const ZERO: Self = Self {
        width: 0,
        height: 0,
    };

    /// Crea un tamaño con `width` y `height` en píxeles.
    #[must_use]
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

/// Rectángulo alineado a los ejes en coordenadas físicas enteras.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rect {
    /// Coordenada X de la esquina superior izquierda, en píxeles físicos.
    pub x: i32,
    /// Coordenada Y de la esquina superior izquierda, en píxeles físicos.
    pub y: i32,
    /// Ancho del rectángulo, en píxeles físicos.
    pub width: u32,
    /// Alto del rectángulo, en píxeles físicos.
    pub height: u32,
}

impl Rect {
    /// Rectángulo nulo en el origen (`0, 0, 0, 0`).
    pub const ZERO: Self = Self {
        x: 0,
        y: 0,
        width: 0,
        height: 0,
    };

    /// Crea un rectángulo en `(x, y)` con dimensiones `width` × `height`.
    #[must_use]
    pub const fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Crea un rectángulo ubicado en `(0, 0)` con el tamaño `size`.
    #[must_use]
    pub const fn from_size(size: Size) -> Self {
        Self {
            x: 0,
            y: 0,
            width: size.width,
            height: size.height,
        }
    }

    /// Tamaño (`width`, `height`) del rectángulo.
    #[must_use]
    pub const fn size(self) -> Size {
        Size::new(self.width, self.height)
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

    /// Comprueba si el punto `(px, py)` cae dentro del rectángulo `[x, x + width) × [y, y + height)`.
    #[must_use]
    pub const fn contains(self, px: i32, py: i32) -> bool {
        if self.width == 0 || self.height == 0 || px < self.x || py < self.y {
            return false;
        }
        let dx = px.abs_diff(self.x);
        let dy = py.abs_diff(self.y);
        dx < self.width && dy < self.height
    }

    /// Comprueba si `inner` está íntegramente contenido dentro de `self`.
    #[must_use]
    pub const fn contains_rect(self, inner: Self) -> bool {
        inner.x >= self.x
            && inner.y >= self.y
            && inner.right() <= self.right()
            && inner.bottom() <= self.bottom()
    }

    /// Calcula la intersección entre `self` y `other`.
    ///
    /// Si no se solapan o alguno tiene área nula, devuelve [`Rect::ZERO`].
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

/// Color RGBA directo de 8 bits por canal (sin premultiplicar), usado en [`Tone::Custom`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rgba {
    /// Canal rojo (`0..=255`).
    pub r: u8,
    /// Canal verde (`0..=255`).
    pub g: u8,
    /// Canal azul (`0..=255`).
    pub b: u8,
    /// Canal alfa (`0..=255`).
    pub a: u8,
}

impl Rgba {
    /// Crea un color opaco (`a = 255`).
    #[must_use]
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// Crea un color con canal alfa explícito.
    #[must_use]
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
}

/// Identificador estable de un widget dentro de un árbol.
///
/// El valor `0` ([`WidgetId::NONE`]) se reserva para nodos decorativos o contenedores
/// que no reciben interacción directa.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct WidgetId(pub u32);

impl WidgetId {
    /// Identificador nulo para widgets sin interacción.
    pub const NONE: Self = Self(0);

    /// Crea un [`WidgetId`] a partir de un entero de 32 bits.
    #[must_use]
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    /// Devuelve `true` si es [`WidgetId::NONE`] (`0`).
    #[must_use]
    pub const fn is_none(self) -> bool {
        self.0 == 0
    }
}

/// Proporción entera en milésimas entre `0` (`0,0 %`) y `1000` (`100,0 %`).
///
/// Evita usar `f32` dentro del árbol de widgets para preservar `Eq` exacto y una
/// serialización determinista por IPC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Permille(u16);

impl Permille {
    /// Valor mínimo (`0` = 0 %).
    pub const ZERO: Self = Self(0);

    /// Valor máximo (`1000` = 100 %).
    pub const FULL: Self = Self(1000);

    /// Límite superior permitido en milésimas (`1000`).
    pub const MAX_RAW: u16 = 1000;

    /// Crea un [`Permille`] saturando cualquier valor mayor a `1000`.
    #[must_use]
    pub const fn new(value: u16) -> Self {
        if value > Self::MAX_RAW {
            Self::FULL
        } else {
            Self(value)
        }
    }

    /// Intenta crear un [`Permille`] validando que `value <= 1000`.
    #[must_use]
    pub const fn try_new(value: u16) -> Option<Self> {
        if value <= Self::MAX_RAW {
            Some(Self(value))
        } else {
            None
        }
    }

    /// Construye un [`Permille`] sin saturar (útil al deserializar antes de [`validate_tree`]).
    #[must_use]
    pub const fn from_raw(raw: u16) -> Self {
        Self(raw)
    }

    /// Devuelve el valor entero en milésimas.
    #[must_use]
    pub const fn raw(self) -> u16 {
        self.0
    }

    /// Convierte el valor acotado a fracción en punto flotante `0.0..=1.0`.
    #[must_use]
    pub fn as_ratio(self) -> f32 {
        f32::from(self.0.min(Self::MAX_RAW)) / 1000.0
    }
}

/// Catálogo de íconos vectoriales predefinidos de `atoll-ui`.
///
/// Es `#[non_exhaustive]` para permitir agregar íconos en versiones futuras sin
/// romper compatibilidad; cualquier variante desconocida se tratará como un
/// marcador neutro al pintar (ítem 14c).
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IconId {
    /// Reproducir (`Play`).
    Play,
    /// Pausar (`Pause`).
    Pause,
    /// Pista siguiente (`NextTrack`).
    NextTrack,
    /// Pista anterior (`PreviousTrack`).
    PreviousTrack,
    /// Volumen activo (`Volume`).
    Volume,
    /// Volumen silenciado (`VolumeMute`).
    VolumeMute,
    /// Engranaje de configuración (`Settings`).
    Settings,
    /// Marca de confirmación (`Check`).
    Check,
    /// Cruz de cierre (`Close`).
    Close,
    /// Flecha / chevron hacia la derecha (`ChevronRight`).
    ChevronRight,
    /// Flecha / chevron hacia abajo (`ChevronDown`).
    ChevronDown,
}

/// Modo de dimensionamiento de un nodo en un eje (modelo flex simple).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Length {
    /// Se ajusta al tamaño intrínseco del contenido del widget.
    #[default]
    Auto,
    /// Tamaño fijo en píxeles lógicos a 96 DPI (`0..=4096`).
    Fixed(u16),
    /// Reparte el espacio sobrante del contenedor padre en proporción al peso (`>= 1`).
    Fill(u8),
}

/// Márgenes internos (padding) en píxeles lógicos a 96 DPI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Edges {
    /// Margen superior lógico.
    pub top: u16,
    /// Margen derecho lógico.
    pub right: u16,
    /// Margen inferior lógico.
    pub bottom: u16,
    /// Margen izquierdo lógico.
    pub left: u16,
}

impl Edges {
    /// Sin márgenes (`0` en los cuatro lados).
    pub const ZERO: Self = Self {
        top: 0,
        right: 0,
        bottom: 0,
        left: 0,
    };

    /// Mismo margen `px` en los cuatro lados.
    #[must_use]
    pub const fn all(px: u16) -> Self {
        Self {
            top: px,
            right: px,
            bottom: px,
            left: px,
        }
    }

    /// Margen simétrico `horizontal` (izquierda/derecha) y `vertical` (arriba/abajo).
    #[must_use]
    pub const fn symmetric(horizontal: u16, vertical: u16) -> Self {
        Self {
            top: vertical,
            right: horizontal,
            bottom: vertical,
            left: horizontal,
        }
    }

    /// Crea márgenes explícitos para cada lado (`top`, `right`, `bottom`, `left`).
    #[must_use]
    pub const fn new(top: u16, right: u16, bottom: u16, left: u16) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }
}

/// Alineación en el eje principal de un `Row` o `Column` cuando sobra espacio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MainAlign {
    /// Agrupa los hijos al inicio del eje principal.
    #[default]
    Start,
    /// Centra el bloque de hijos en el eje principal.
    Center,
    /// Agrupa los hijos al final del eje principal.
    End,
    /// Distribuye el espacio libre de forma uniforme entre los hijos.
    SpaceBetween,
}

/// Alineación en el eje transversal de un `Row` o `Column`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CrossAlign {
    /// Alinea al borde inicial transversal (arriba en `Row`, izquierda en `Column`).
    #[default]
    Start,
    /// Centra en el eje transversal.
    Center,
    /// Alinea al borde final transversal (abajo en `Row`, derecha en `Column`).
    End,
    /// Estira los hijos con tamaño transversal `Auto` para ocupar todo el eje transversal.
    Stretch,
}

/// Rol semántico de color para texto, íconos y barras de progreso.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tone {
    /// Color principal de texto del tema activo.
    #[default]
    Primary,
    /// Color secundario / atenuado del tema activo.
    Muted,
    /// Color de acento derivado del preset del tema activo.
    Accent,
    /// Color explícito RGBA.
    Custom(Rgba),
}

/// Variante visual de un [`WidgetKind::Button`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonVariant {
    /// Botón secundario con superficie sutilmente elevada.
    #[default]
    Secondary,
    /// Botón principal destacado con el color de acento.
    Primary,
    /// Botón sin fondo en reposo (muestra fondo solo al pasar o presionar el mouse).
    Ghost,
}

/// Nodo del árbol declarativo de widgets.
///
/// Es 100 % datos planos (`Clone + Debug + PartialEq + Eq`), sin closures ni referencias.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    /// Identificador del widget (`WidgetId::NONE` si no es interactivo).
    pub id: WidgetId,
    /// Regla de ancho lógico.
    pub width: Length,
    /// Regla de alto lógico.
    pub height: Length,
    /// Relleno interno lógico.
    pub padding: Edges,
    /// Especie del widget y sus propiedades específicas.
    pub kind: WidgetKind,
}

/// Los widgets soportados por `atoll-ui` (v1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WidgetKind {
    /// Texto de una sola línea.
    Label {
        /// Contenido de texto a mostrar.
        text: String,
        /// Tamaño de fuente lógico en píxeles a 96 DPI (`1..=4096`).
        size_px: u16,
        /// Tono de color del texto.
        tone: Tone,
    },
    /// Ícono vectorial predefinido.
    Icon {
        /// Identificador del ícono del catálogo.
        icon: IconId,
        /// Tamaño lógico del cuadro del ícono en píxeles a 96 DPI (`1..=4096`).
        size_px: u16,
        /// Tono de color del ícono.
        tone: Tone,
    },
    /// Contenedor horizontal que dispone a sus hijos de izquierda a derecha.
    Row {
        /// Separación lógica entre hijos consecutivos, en píxeles a 96 DPI.
        gap: u16,
        /// Alineación en el eje horizontal.
        main_align: MainAlign,
        /// Alineación en el eje vertical.
        cross_align: CrossAlign,
        /// Hijos del contenedor.
        children: Vec<Node>,
    },
    /// Contenedor vertical que dispone a sus hijos de arriba hacia abajo.
    Column {
        /// Separación lógica entre hijos consecutivos, en píxeles a 96 DPI.
        gap: u16,
        /// Alineación en el eje vertical.
        main_align: MainAlign,
        /// Alineación en el eje horizontal.
        cross_align: CrossAlign,
        /// Hijos del contenedor.
        children: Vec<Node>,
    },
    /// Espacio vacío fijo o elástico dentro de un `Row` o `Column`.
    Spacer,
    /// Botón interactivo con etiqueta opcional e ícono opcional.
    Button {
        /// Texto opcional del botón.
        label: Option<String>,
        /// Ícono opcional del botón.
        icon: Option<IconId>,
        /// Estilo visual del botón.
        variant: ButtonVariant,
        /// `true` si acepta clics e interacción.
        enabled: bool,
    },
    /// Interruptor booleano de dos estados.
    Toggle {
        /// Estado actual (`true` = activado, `false` = desactivado).
        checked: bool,
        /// `true` si acepta clics e interacción.
        enabled: bool,
    },
    /// Control deslizable continuo con valor en milésimas (`0..=1000`).
    Slider {
        /// Valor normalizado actual (`0..=1000`).
        value: Permille,
        /// `true` si acepta clics y arrastre.
        enabled: bool,
    },
    /// Barra de progreso de solo lectura con valor en milésimas (`0..=1000`).
    Progress {
        /// Progreso normalizado actual (`0..=1000`).
        value: Permille,
        /// Tono de la barra rellena.
        tone: Tone,
    },
    /// Línea divisoria dentro de un `Row` (vertical) o `Column` (horizontal).
    Separator {
        /// Espesor lógico de la línea en píxeles a 96 DPI (`1..=4096`).
        thickness_px: u16,
    },
}

impl Node {
    /// Crea un nodo con `WidgetId::NONE`, tamaño `Auto` en ambos ejes y sin padding.
    #[must_use]
    pub const fn new(kind: WidgetKind) -> Self {
        Self {
            id: WidgetId::NONE,
            width: Length::Auto,
            height: Length::Auto,
            padding: Edges::ZERO,
            kind,
        }
    }

    /// Crea un nodo [`WidgetKind::Label`].
    #[must_use]
    pub fn label(text: impl Into<String>, size_px: u16, tone: Tone) -> Self {
        Self::new(WidgetKind::Label {
            text: text.into(),
            size_px,
            tone,
        })
    }

    /// Crea un nodo [`WidgetKind::Icon`].
    #[must_use]
    pub const fn icon(icon: IconId, size_px: u16, tone: Tone) -> Self {
        Self::new(WidgetKind::Icon {
            icon,
            size_px,
            tone,
        })
    }

    /// Crea un contenedor horizontal [`WidgetKind::Row`].
    #[must_use]
    pub const fn row(
        gap: u16,
        main_align: MainAlign,
        cross_align: CrossAlign,
        children: Vec<Node>,
    ) -> Self {
        Self::new(WidgetKind::Row {
            gap,
            main_align,
            cross_align,
            children,
        })
    }

    /// Crea un contenedor vertical [`WidgetKind::Column`].
    #[must_use]
    pub const fn column(
        gap: u16,
        main_align: MainAlign,
        cross_align: CrossAlign,
        children: Vec<Node>,
    ) -> Self {
        Self::new(WidgetKind::Column {
            gap,
            main_align,
            cross_align,
            children,
        })
    }

    /// Crea un contenedor con relleno interno (`padding`) y sus hijos apilados verticalmente sin separación.
    #[must_use]
    pub const fn box_container(padding: Edges, children: Vec<Node>) -> Self {
        Self {
            id: WidgetId::NONE,
            width: Length::Auto,
            height: Length::Auto,
            padding,
            kind: WidgetKind::Column {
                gap: 0,
                main_align: MainAlign::Start,
                cross_align: CrossAlign::Start,
                children,
            },
        }
    }

    /// Crea un [`WidgetKind::Spacer`] que rellena el eje principal con peso `weight`.
    #[must_use]
    pub const fn spacer_fill(weight: u8) -> Self {
        Self {
            id: WidgetId::NONE,
            width: Length::Fill(weight),
            height: Length::Fill(weight),
            padding: Edges::ZERO,
            kind: WidgetKind::Spacer,
        }
    }

    /// Crea un [`WidgetKind::Spacer`] con ancho y alto lógicos fijos.
    #[must_use]
    pub const fn spacer_fixed(width: u16, height: u16) -> Self {
        Self {
            id: WidgetId::NONE,
            width: Length::Fixed(width),
            height: Length::Fixed(height),
            padding: Edges::ZERO,
            kind: WidgetKind::Spacer,
        }
    }

    /// Crea un [`WidgetKind::Button`].
    #[must_use]
    pub const fn button(
        id: WidgetId,
        label: Option<String>,
        icon: Option<IconId>,
        variant: ButtonVariant,
        enabled: bool,
    ) -> Self {
        Self {
            id,
            width: Length::Auto,
            height: Length::Auto,
            padding: Edges::ZERO,
            kind: WidgetKind::Button {
                label,
                icon,
                variant,
                enabled,
            },
        }
    }

    /// Crea un botón de solo ícono ([`WidgetKind::Button`] con `label: None`).
    #[must_use]
    pub const fn icon_button(
        id: WidgetId,
        icon: IconId,
        variant: ButtonVariant,
        enabled: bool,
    ) -> Self {
        Self::button(id, None, Some(icon), variant, enabled)
    }

    /// Crea un [`WidgetKind::Toggle`].
    #[must_use]
    pub const fn toggle(id: WidgetId, checked: bool, enabled: bool) -> Self {
        Self {
            id,
            width: Length::Auto,
            height: Length::Auto,
            padding: Edges::ZERO,
            kind: WidgetKind::Toggle { checked, enabled },
        }
    }

    /// Crea un [`WidgetKind::Slider`].
    #[must_use]
    pub const fn slider(id: WidgetId, value: Permille, enabled: bool) -> Self {
        Self {
            id,
            width: Length::Auto,
            height: Length::Auto,
            padding: Edges::ZERO,
            kind: WidgetKind::Slider { value, enabled },
        }
    }

    /// Crea un [`WidgetKind::Progress`].
    #[must_use]
    pub const fn progress(value: Permille, tone: Tone) -> Self {
        Self::new(WidgetKind::Progress { value, tone })
    }

    /// Alias ergonómico de [`Node::progress`].
    #[must_use]
    pub const fn progress_bar(value: Permille, tone: Tone) -> Self {
        Self::progress(value, tone)
    }

    /// Crea un [`WidgetKind::Separator`] con espesor lógico `thickness_px`.
    #[must_use]
    pub const fn separator(thickness_px: u16) -> Self {
        Self::new(WidgetKind::Separator { thickness_px })
    }

    /// Alias ergonómico de [`Node::separator`].
    #[must_use]
    pub const fn divider(thickness_px: u16) -> Self {
        Self::separator(thickness_px)
    }

    /// Asigna el identificador [`WidgetId`] al nodo.
    #[must_use]
    pub const fn with_id(mut self, id: WidgetId) -> Self {
        self.id = id;
        self
    }

    /// Asigna la regla de ancho lógico al nodo.
    #[must_use]
    pub const fn with_width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }

    /// Asigna la regla de alto lógico al nodo.
    #[must_use]
    pub const fn with_height(mut self, height: Length) -> Self {
        self.height = height;
        self
    }

    /// Asigna ancho y alto lógicos fijos al nodo.
    #[must_use]
    pub const fn with_fixed_size(mut self, width: u16, height: u16) -> Self {
        self.width = Length::Fixed(width);
        self.height = Length::Fixed(height);
        self
    }

    /// Asigna el relleno interno lógico (`padding`) al nodo.
    #[must_use]
    pub const fn with_padding(mut self, padding: Edges) -> Self {
        self.padding = padding;
        self
    }
}

/// Estadísticas estructurales devueltas al validar un árbol con [`validate_tree`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TreeStats {
    /// Cantidad total de nodos en el árbol (`1..=MAX_TREE_NODES`).
    pub node_count: usize,
    /// Profundidad máxima alcanzada (`1..=MAX_TREE_DEPTH`).
    pub max_depth: usize,
}

/// Errores de validación de un árbol de widgets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreeError {
    /// El árbol supera la profundidad máxima permitida (`MAX_TREE_DEPTH = 32`).
    MaxDepthExceeded {
        /// Profundidad máxima permitida.
        max: usize,
    },
    /// El árbol supera la cantidad máxima de nodos permitida (`MAX_TREE_NODES = 1024`).
    MaxNodesExceeded {
        /// Cantidad máxima de nodos permitida.
        max: usize,
    },
    /// Dos o más nodos usan el mismo [`WidgetId`] distinto de [`WidgetId::NONE`].
    DuplicateWidgetId(WidgetId),
    /// Un nodo usa `Length::Fill(0)` (el peso de relleno debe ser `>= 1`).
    InvalidFillWeight {
        /// Identificador del nodo con peso inválido.
        id: WidgetId,
    },
    /// Un campo numérico del nodo está fuera de su rango permitido.
    ValueOutOfRange {
        /// Identificador del nodo donde ocurrió el error.
        id: WidgetId,
        /// Nombre del campo inválido.
        field: &'static str,
        /// Valor encontrado.
        value: u32,
        /// Valor mínimo permitido (inclusive).
        min: u32,
        /// Valor máximo permitido (inclusive).
        max: u32,
    },
}

impl fmt::Display for TreeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MaxDepthExceeded { max } => {
                write!(f, "el árbol supera la profundidad máxima permitida ({max})")
            }
            Self::MaxNodesExceeded { max } => {
                write!(
                    f,
                    "el árbol supera la cantidad máxima de nodos permitida ({max})"
                )
            }
            Self::DuplicateWidgetId(id) => {
                write!(f, "identificador de widget duplicado: {}", id.0)
            }
            Self::InvalidFillWeight { id } => {
                write!(f, "el peso de Fill debe ser >= 1 (widget {})", id.0)
            }
            Self::ValueOutOfRange {
                id,
                field,
                value,
                min,
                max,
            } => write!(
                f,
                "valor fuera de rango en '{field}' (widget {}): {value} (permitido {min}..={max})",
                id.0
            ),
        }
    }
}

impl std::error::Error for TreeError {}

/// Valida iterativamente un árbol de widgets sin riesgo de desborde de pila.
///
/// Verifica:
/// - Profundidad máxima [`MAX_TREE_DEPTH`] (`32`).
/// - Cantidad máxima de nodos [`MAX_TREE_NODES`] (`1024`).
/// - Ausencia de [`WidgetId`] duplicados (ignorando [`WidgetId::NONE`]).
/// - Pesos `Length::Fill(w)` con `w >= 1` y medidas lógicas dentro de rango (`<= MAX_LOGICAL_PX`).
/// - Valores [`Permille`] en `0..=1000`.
///
/// # Errors
///
/// Devuelve [`TreeError`] en cuanto detecta la primera violación.
pub fn validate_tree(root: &Node) -> Result<TreeStats, TreeError> {
    let mut stack: Vec<(&Node, usize)> = Vec::with_capacity(32);
    stack.push((root, 1));

    let mut seen_ids: Vec<WidgetId> = Vec::new();
    let mut node_count: usize = 0;
    let mut max_depth: usize = 0;

    while let Some((node, depth)) = stack.pop() {
        if depth > MAX_TREE_DEPTH {
            return Err(TreeError::MaxDepthExceeded {
                max: MAX_TREE_DEPTH,
            });
        }
        node_count = node_count.saturating_add(1);
        if node_count > MAX_TREE_NODES {
            return Err(TreeError::MaxNodesExceeded {
                max: MAX_TREE_NODES,
            });
        }
        if depth > max_depth {
            max_depth = depth;
        }

        if !node.id.is_none() {
            if seen_ids.contains(&node.id) {
                return Err(TreeError::DuplicateWidgetId(node.id));
            }
            seen_ids.push(node.id);
        }

        validate_length(node.id, "width", node.width)?;
        validate_length(node.id, "height", node.height)?;
        validate_max_px(node.id, "padding.top", node.padding.top)?;
        validate_max_px(node.id, "padding.right", node.padding.right)?;
        validate_max_px(node.id, "padding.bottom", node.padding.bottom)?;
        validate_max_px(node.id, "padding.left", node.padding.left)?;

        match &node.kind {
            WidgetKind::Label { size_px, .. } | WidgetKind::Icon { size_px, .. } => {
                validate_non_zero_px(node.id, "size_px", *size_px)?;
            }
            WidgetKind::Row { gap, children, .. } | WidgetKind::Column { gap, children, .. } => {
                validate_max_px(node.id, "gap", *gap)?;
                let next_depth = depth.saturating_add(1);
                // Apilamos en orden inverso para recorrer a los hijos en su orden natural.
                for child in children.iter().rev() {
                    stack.push((child, next_depth));
                }
            }
            WidgetKind::Spacer | WidgetKind::Button { .. } | WidgetKind::Toggle { .. } => {}
            WidgetKind::Slider { value, .. } | WidgetKind::Progress { value, .. } => {
                if value.raw() > Permille::MAX_RAW {
                    return Err(TreeError::ValueOutOfRange {
                        id: node.id,
                        field: "value",
                        value: u32::from(value.raw()),
                        min: 0,
                        max: u32::from(Permille::MAX_RAW),
                    });
                }
            }
            WidgetKind::Separator { thickness_px } => {
                validate_non_zero_px(node.id, "thickness_px", *thickness_px)?;
            }
        }
    }

    Ok(TreeStats {
        node_count,
        max_depth,
    })
}

fn validate_length(id: WidgetId, field: &'static str, length: Length) -> Result<(), TreeError> {
    match length {
        Length::Auto => Ok(()),
        Length::Fixed(px) => validate_max_px(id, field, px),
        Length::Fill(0) => Err(TreeError::InvalidFillWeight { id }),
        Length::Fill(_) => Ok(()),
    }
}

fn validate_max_px(id: WidgetId, field: &'static str, px: u16) -> Result<(), TreeError> {
    if px > MAX_LOGICAL_PX {
        return Err(TreeError::ValueOutOfRange {
            id,
            field,
            value: u32::from(px),
            min: 0,
            max: u32::from(MAX_LOGICAL_PX),
        });
    }
    Ok(())
}

fn validate_non_zero_px(id: WidgetId, field: &'static str, px: u16) -> Result<(), TreeError> {
    if px == 0 || px > MAX_LOGICAL_PX {
        return Err(TreeError::ValueOutOfRange {
            id,
            field,
            value: u32::from(px),
            min: 1,
            max: u32::from(MAX_LOGICAL_PX),
        });
    }
    Ok(())
}

/// Abstracción mínima para medir texto durante el cálculo de layout.
///
/// Permite usar `fontdue` cuando se integra con `atoll-render` y un medidor
/// determinista liviano en tests unitarios.
pub trait TextMeasurer {
    /// Devuelve el tamaño físico en píxeles (`Size`) que ocupa `text` al
    /// renderizarse con altura de cuerpo física `size_px` (en píxeles).
    fn measure_line(&mut self, text: &str, size_px: u32) -> Size;
}

/// Datos específicos de un nodo ya resueltos a coordenadas físicas en el [`LayoutTree`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutKind {
    /// Etiqueta de texto con su tamaño físico de fuente ya escalado.
    Label {
        /// Texto a dibujar.
        text: String,
        /// Tamaño físico de la fuente en píxeles.
        size_px: u32,
        /// Tono de color.
        tone: Tone,
    },
    /// Ícono vectorial con su tamaño físico en píxeles.
    Icon {
        /// Identificador del ícono.
        icon: IconId,
        /// Tamaño físico en píxeles.
        size_px: u32,
        /// Tono de color.
        tone: Tone,
    },
    /// Contenedor horizontal o vertical con los índices de sus hijos dentro de [`LayoutTree::nodes`].
    Container {
        /// Índices de los hijos directos dentro de `LayoutTree::nodes`.
        children: Vec<usize>,
    },
    /// Espacio vacío sin representación visual.
    Spacer,
    /// Botón interactivo posicionado.
    Button {
        /// Texto opcional.
        label: Option<String>,
        /// Ícono opcional.
        icon: Option<IconId>,
        /// Variante visual.
        variant: ButtonVariant,
        /// Si está habilitado.
        enabled: bool,
    },
    /// Interruptor booleano posicionado.
    Toggle {
        /// Estado actual.
        checked: bool,
        /// Si está habilitado.
        enabled: bool,
    },
    /// Control deslizable posicionado.
    Slider {
        /// Valor normalizado (`0..=1000`).
        value: Permille,
        /// Si está habilitado.
        enabled: bool,
    },
    /// Barra de progreso posicionada.
    Progress {
        /// Valor normalizado (`0..=1000`).
        value: Permille,
        /// Tono de color.
        tone: Tone,
    },
    /// Línea divisoria posicionada.
    Separator,
}

/// Nodo posicionado en píxeles físicos dentro de [`LayoutTree`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutNode {
    /// Identificador del widget (`WidgetId::NONE` si no es interactivo).
    pub id: WidgetId,
    /// Caja externa física del widget (incluye `padding`), recortada al contenedor padre.
    pub rect: Rect,
    /// Caja interna de contenido física (`rect` menos `padding` escalado), siempre dentro de `rect`.
    pub content_rect: Rect,
    /// Especie resuelta del nodo.
    pub kind: LayoutKind,
}

/// Resultado plano y determinista de aplicar layout a un árbol [`Node`].
///
/// Todos los nodos quedan almacenados en preorden dentro de `nodes` (el índice `0`
/// es siempre la raíz).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutTree {
    /// Tamaño físico disponible sobre el que se calculó el layout.
    pub available: Size,
    /// Factor de escala utilizado, en milésimas (`1000` = 1,0x).
    pub scale_milli: u32,
    /// Lista plana de nodos posicionados en preorden (`nodes[0]` es la raíz).
    pub nodes: Vec<LayoutNode>,
}

impl LayoutTree {
    /// Devuelve el nodo raíz posicionado, si existe.
    #[must_use]
    pub fn root(&self) -> Option<&LayoutNode> {
        self.nodes.first()
    }

    /// Busca un nodo posicionado por su [`WidgetId`].
    #[must_use]
    pub fn find_by_id(&self, id: WidgetId) -> Option<&LayoutNode> {
        if id.is_none() {
            return None;
        }
        self.nodes.iter().find(|node| node.id == id)
    }
}

/// Convierte un factor de escala `f32` (por ejemplo `1.25`) a milésimas enteras (`1250`).
///
/// Cualquier valor no finito o no positivo se normaliza a `1000` (`1,0x`), y el
/// resultado se acota al rango `250..=8000` (`0,25x..=8,0x`).
#[must_use]
pub fn normalize_scale_milli(scale: f32) -> u32 {
    if !scale.is_finite() || scale <= 0.0 {
        return DEFAULT_SCALE_MILLI;
    }
    let clamped = scale.clamp(0.25, 8.0);
    let milli = (clamped * 1000.0).round() as u32;
    milli.clamp(MIN_SCALE_MILLI, MAX_SCALE_MILLI)
}

/// Escala una medida lógica `px` (a 96 DPI) según `scale_milli` (`1000` = 1,0x),
/// redondeando al píxel entero más cercano sin desborde.
#[must_use]
pub const fn scale_logical_px(px: u16, scale_milli: u32) -> u32 {
    let prod = (px as u64).saturating_mul(scale_milli as u64);
    ((prod + 500) / 1000) as u32
}

/// Calcula el layout físico de `root` dentro de la superficie `available` al factor de escala `scale`.
///
/// # Regla de recorte ante desborde
///
/// Ningún hijo puede salirse nunca del `content_rect` (ni del `rect`) de su contenedor padre.
/// Si la suma de tamaños `Fixed` o `Auto` de los hijos supera el espacio disponible del padre,
/// cada hijo se posiciona secuencialmente y su ancho/alto se recorta al espacio remanente
/// (`min(remaining_space)`). Los hijos que quedan después de agotarse el espacio reciben
/// tamaño `0` en el borde final del `content_rect` del padre.
///
/// # Errors
///
/// Devuelve [`TreeError`] si `validate_tree(root)` detecta que el árbol supera los límites de
/// profundidad o cantidad de nodos, tiene `WidgetId` duplicados o valores fuera de rango.
pub fn compute_layout(
    root: &Node,
    available: Size,
    scale: f32,
    measurer: &mut dyn TextMeasurer,
) -> Result<LayoutTree, TreeError> {
    compute_layout_milli(root, available, normalize_scale_milli(scale), measurer)
}

/// Igual que [`compute_layout`], recibiendo la escala directamente en milésimas (`1000` = 1,0x).
///
/// # Errors
///
/// Devuelve [`TreeError`] si `root` no pasa [`validate_tree`].
pub fn compute_layout_milli(
    root: &Node,
    available: Size,
    scale_milli: u32,
    measurer: &mut dyn TextMeasurer,
) -> Result<LayoutTree, TreeError> {
    let stats = validate_tree(root)?;
    let scale_milli = if scale_milli == 0 {
        DEFAULT_SCALE_MILLI
    } else {
        scale_milli.clamp(MIN_SCALE_MILLI, MAX_SCALE_MILLI)
    };

    // Pasada 1 (bottom-up): calculamos el tamaño físico intrínseco de cada nodo en preorden.
    let mut intrinsic_sizes = vec![Size::ZERO; stats.node_count];
    compute_intrinsics_recursive(root, scale_milli, measurer, &mut 0, &mut intrinsic_sizes);

    // Determinamos el rectángulo físico de la raíz dentro de `available`.
    let root_intrinsic = intrinsic_sizes.first().copied().unwrap_or(Size::ZERO);
    let root_w = resolve_root_dim(
        root.width,
        root_intrinsic.width,
        available.width,
        scale_milli,
    );
    let root_h = resolve_root_dim(
        root.height,
        root_intrinsic.height,
        available.height,
        scale_milli,
    );
    let root_rect = Rect::new(0, 0, root_w, root_h);

    // Pasada 2 (top-down): ubicamos cada nodo en preorden dentro de su rectángulo asignado.
    let mut nodes = Vec::with_capacity(stats.node_count);
    place_node_recursive(
        root,
        root_rect,
        scale_milli,
        &intrinsic_sizes,
        &mut 0,
        &mut nodes,
    );

    Ok(LayoutTree {
        available,
        scale_milli,
        nodes,
    })
}

fn resolve_root_dim(rule: Length, intrinsic: u32, available: u32, scale_milli: u32) -> u32 {
    match rule {
        Length::Auto => intrinsic.min(available),
        Length::Fixed(px) => scale_logical_px(px, scale_milli).min(available),
        Length::Fill(_) => available,
    }
}

/// Calcula recursivamente (con profundidad ya validada `<= MAX_TREE_DEPTH`) el tamaño
/// físico intrínseco de cada nodo y lo guarda en `out[preorder_index]`.
fn compute_intrinsics_recursive(
    node: &Node,
    scale_milli: u32,
    measurer: &mut dyn TextMeasurer,
    cursor: &mut usize,
    out: &mut [Size],
) -> Size {
    let my_index = *cursor;
    *cursor = cursor.saturating_add(1);

    let pad_h = scale_logical_px(node.padding.left, scale_milli)
        .saturating_add(scale_logical_px(node.padding.right, scale_milli));
    let pad_v = scale_logical_px(node.padding.top, scale_milli)
        .saturating_add(scale_logical_px(node.padding.bottom, scale_milli));

    let content_size = match &node.kind {
        WidgetKind::Label { text, size_px, .. } => {
            let phys_px = scale_logical_px(*size_px, scale_milli).max(1);
            measurer.measure_line(text, phys_px)
        }
        WidgetKind::Icon { size_px, .. } => {
            let side = scale_logical_px(*size_px, scale_milli);
            Size::new(side, side)
        }
        WidgetKind::Row { gap, children, .. } => {
            let gap_phys = scale_logical_px(*gap, scale_milli);
            let mut total_w: u32 = 0;
            let mut max_h: u32 = 0;
            for (idx, child) in children.iter().enumerate() {
                let child_intr =
                    compute_intrinsics_recursive(child, scale_milli, measurer, cursor, out);
                if idx > 0 {
                    total_w = total_w.saturating_add(gap_phys);
                }
                total_w = total_w.saturating_add(child_intr.width);
                if child_intr.height > max_h {
                    max_h = child_intr.height;
                }
            }
            Size::new(total_w, max_h)
        }
        WidgetKind::Column { gap, children, .. } => {
            let gap_phys = scale_logical_px(*gap, scale_milli);
            let mut max_w: u32 = 0;
            let mut total_h: u32 = 0;
            for (idx, child) in children.iter().enumerate() {
                let child_intr =
                    compute_intrinsics_recursive(child, scale_milli, measurer, cursor, out);
                if idx > 0 {
                    total_h = total_h.saturating_add(gap_phys);
                }
                total_h = total_h.saturating_add(child_intr.height);
                if child_intr.width > max_w {
                    max_w = child_intr.width;
                }
            }
            Size::new(max_w, total_h)
        }
        WidgetKind::Spacer => Size::ZERO,
        WidgetKind::Button { label, icon, .. } => {
            let icon_w = if icon.is_some() {
                scale_logical_px(BUTTON_ICON_PX, scale_milli)
            } else {
                0
            };
            let text_size = match label.as_deref() {
                Some(text) if !text.is_empty() => {
                    let phys_px = scale_logical_px(BUTTON_TEXT_PX, scale_milli).max(1);
                    measurer.measure_line(text, phys_px)
                }
                _ => Size::ZERO,
            };
            let inner_gap = if icon.is_some() && text_size.width > 0 {
                scale_logical_px(BUTTON_GAP_PX, scale_milli)
            } else {
                0
            };
            let btn_pad_x = scale_logical_px(BUTTON_PAD_X_PX, scale_milli).saturating_mul(2);
            let btn_pad_y = scale_logical_px(BUTTON_PAD_Y_PX, scale_milli).saturating_mul(2);
            let min_h = scale_logical_px(BUTTON_MIN_HEIGHT_PX, scale_milli);
            let w = icon_w
                .saturating_add(inner_gap)
                .saturating_add(text_size.width)
                .saturating_add(btn_pad_x);
            let h = text_size
                .height
                .max(icon_w)
                .saturating_add(btn_pad_y)
                .max(min_h);
            Size::new(w, h)
        }
        WidgetKind::Toggle { .. } => Size::new(
            scale_logical_px(TOGGLE_WIDTH_PX, scale_milli),
            scale_logical_px(TOGGLE_HEIGHT_PX, scale_milli),
        ),
        WidgetKind::Slider { .. } => Size::new(
            scale_logical_px(SLIDER_MIN_WIDTH_PX, scale_milli),
            scale_logical_px(SLIDER_HEIGHT_PX, scale_milli),
        ),
        WidgetKind::Progress { .. } => Size::new(
            scale_logical_px(PROGRESS_MIN_WIDTH_PX, scale_milli),
            scale_logical_px(PROGRESS_HEIGHT_PX, scale_milli),
        ),
        WidgetKind::Separator { thickness_px } => {
            let t = scale_logical_px(*thickness_px, scale_milli).max(1);
            Size::new(t, t)
        }
    };

    let raw_w = content_size.width.saturating_add(pad_h);
    let raw_h = content_size.height.saturating_add(pad_v);

    let final_w = match node.width {
        Length::Fixed(px) => scale_logical_px(px, scale_milli),
        Length::Auto | Length::Fill(_) => raw_w,
    };
    let final_h = match node.height {
        Length::Fixed(px) => scale_logical_px(px, scale_milli),
        Length::Auto | Length::Fill(_) => raw_h,
    };

    let result = Size::new(final_w, final_h);
    if let Some(slot) = out.get_mut(my_index) {
        *slot = result;
    }
    result
}

/// Posiciona recursivamente `node` dentro de `rect` y agrega el [`LayoutNode`] a `out`.
fn place_node_recursive(
    node: &Node,
    rect: Rect,
    scale_milli: u32,
    intrinsic_sizes: &[Size],
    cursor: &mut usize,
    out: &mut Vec<LayoutNode>,
) -> usize {
    let my_preorder = *cursor;
    *cursor = cursor.saturating_add(1);

    let pad_left = scale_logical_px(node.padding.left, scale_milli);
    let pad_right = scale_logical_px(node.padding.right, scale_milli);
    let pad_top = scale_logical_px(node.padding.top, scale_milli);
    let pad_bottom = scale_logical_px(node.padding.bottom, scale_milli);

    let content_x = rect.x.saturating_add_unsigned(pad_left.min(rect.width));
    let content_y = rect.y.saturating_add_unsigned(pad_top.min(rect.height));
    let content_w = rect
        .width
        .saturating_sub(pad_left)
        .saturating_sub(pad_right);
    let content_h = rect
        .height
        .saturating_sub(pad_top)
        .saturating_sub(pad_bottom);
    let content_rect = Rect::new(content_x, content_y, content_w, content_h);

    let out_index = out.len();
    // Insertamos primero un nodo marcador para conservar el orden preorden exacto.
    out.push(LayoutNode {
        id: node.id,
        rect,
        content_rect,
        kind: LayoutKind::Spacer,
    });

    let layout_kind = match &node.kind {
        WidgetKind::Label {
            text,
            size_px,
            tone,
        } => LayoutKind::Label {
            text: text.clone(),
            size_px: scale_logical_px(*size_px, scale_milli).max(1),
            tone: *tone,
        },
        WidgetKind::Icon {
            icon,
            size_px,
            tone,
        } => LayoutKind::Icon {
            icon: *icon,
            size_px: scale_logical_px(*size_px, scale_milli),
            tone: *tone,
        },
        WidgetKind::Row {
            gap,
            main_align,
            cross_align,
            children,
        } => {
            let child_indices = place_flex_children(
                children,
                true,
                content_rect,
                scale_logical_px(*gap, scale_milli),
                *main_align,
                *cross_align,
                scale_milli,
                intrinsic_sizes,
                cursor,
                out,
            );
            LayoutKind::Container {
                children: child_indices,
            }
        }
        WidgetKind::Column {
            gap,
            main_align,
            cross_align,
            children,
        } => {
            let child_indices = place_flex_children(
                children,
                false,
                content_rect,
                scale_logical_px(*gap, scale_milli),
                *main_align,
                *cross_align,
                scale_milli,
                intrinsic_sizes,
                cursor,
                out,
            );
            LayoutKind::Container {
                children: child_indices,
            }
        }
        WidgetKind::Spacer => LayoutKind::Spacer,
        WidgetKind::Button {
            label,
            icon,
            variant,
            enabled,
        } => LayoutKind::Button {
            label: label.clone(),
            icon: *icon,
            variant: *variant,
            enabled: *enabled,
        },
        WidgetKind::Toggle { checked, enabled } => LayoutKind::Toggle {
            checked: *checked,
            enabled: *enabled,
        },
        WidgetKind::Slider { value, enabled } => LayoutKind::Slider {
            value: *value,
            enabled: *enabled,
        },
        WidgetKind::Progress { value, tone } => LayoutKind::Progress {
            value: *value,
            tone: *tone,
        },
        WidgetKind::Separator { .. } => LayoutKind::Separator,
    };

    let _ = my_preorder;
    if let Some(slot) = out.get_mut(out_index) {
        slot.kind = layout_kind;
    }
    out_index
}

/// Cuenta cuántos nodos hay en el subárbol de `node` (incluyéndolo).
fn subtree_node_count(node: &Node) -> usize {
    match &node.kind {
        WidgetKind::Row { children, .. } | WidgetKind::Column { children, .. } => {
            children.iter().fold(1usize, |acc, child| {
                acc.saturating_add(subtree_node_count(child))
            })
        }
        _ => 1,
    }
}

#[allow(clippy::too_many_arguments)]
fn place_flex_children(
    children: &[Node],
    is_row: bool,
    content_rect: Rect,
    gap_phys: u32,
    main_align: MainAlign,
    cross_align: CrossAlign,
    scale_milli: u32,
    intrinsic_sizes: &[Size],
    cursor: &mut usize,
    out: &mut Vec<LayoutNode>,
) -> Vec<usize> {
    let n = children.len();
    if n == 0 {
        return Vec::new();
    }

    let main_avail = if is_row {
        content_rect.width
    } else {
        content_rect.height
    };
    let cross_avail = if is_row {
        content_rect.height
    } else {
        content_rect.width
    };

    // Obtenemos el índice preorden de cada hijo directo para consultar su tamaño intrínseco.
    let mut child_preorder = Vec::with_capacity(n);
    let mut scan = *cursor;
    for child in children {
        child_preorder.push(scan);
        scan = scan.saturating_add(subtree_node_count(child));
    }

    // 1. Calculamos tamaños base en el eje principal (Fixed y Auto) y pesos Fill.
    let mut allocated_main = vec![0u32; n];
    let mut fill_weights = vec![0u8; n];
    let mut used_non_fill: u32 = 0;
    let mut total_weight: u64 = 0;

    for (i, child) in children.iter().enumerate() {
        let intr = intrinsic_sizes
            .get(child_preorder[i])
            .copied()
            .unwrap_or(Size::ZERO);
        let main_rule = if is_row { child.width } else { child.height };
        match main_rule {
            Length::Auto => {
                let d = if is_row { intr.width } else { intr.height };
                allocated_main[i] = d;
                used_non_fill = used_non_fill.saturating_add(d);
            }
            Length::Fixed(px) => {
                let d = scale_logical_px(px, scale_milli);
                allocated_main[i] = d;
                used_non_fill = used_non_fill.saturating_add(d);
            }
            Length::Fill(weight) => {
                fill_weights[i] = weight;
                total_weight = total_weight.saturating_add(u64::from(weight));
            }
        }
    }

    let total_gaps = (n.saturating_sub(1) as u32).saturating_mul(gap_phys);
    let remaining_for_fill = main_avail
        .saturating_sub(total_gaps)
        .saturating_sub(used_non_fill);

    // 2. Repartimos el espacio disponible entre los hijos Fill con distribución exacta del resto.
    if total_weight > 0 && remaining_for_fill > 0 {
        let mut distributed: u32 = 0;
        for i in 0..n {
            let w = u64::from(fill_weights[i]);
            if w > 0 {
                let share =
                    ((u64::from(remaining_for_fill).saturating_mul(w)) / total_weight) as u32;
                allocated_main[i] = share;
                distributed = distributed.saturating_add(share);
            }
        }
        let mut rem_px = remaining_for_fill.saturating_sub(distributed);
        if rem_px > 0 {
            for i in 0..n {
                if fill_weights[i] > 0 && rem_px > 0 {
                    allocated_main[i] = allocated_main[i].saturating_add(1);
                    rem_px -= 1;
                }
            }
        }
    }

    // 3. Calculamos el desplazamiento inicial y el espacio extra entre hijos según MainAlign.
    let sum_allocated = allocated_main
        .iter()
        .fold(0u32, |acc, &val| acc.saturating_add(val));
    let total_occupied = sum_allocated.saturating_add(total_gaps);
    let free_main = if total_weight == 0 {
        main_avail.saturating_sub(total_occupied)
    } else {
        0
    };

    let (mut main_cursor, extra_gap_base, mut extra_gap_rem) = match main_align {
        MainAlign::Start => (0u32, 0u32, 0u32),
        MainAlign::Center => (free_main / 2, 0, 0),
        MainAlign::End => (free_main, 0, 0),
        MainAlign::SpaceBetween => {
            if n > 1 {
                let gaps_count = (n - 1) as u32;
                (0, free_main / gaps_count, free_main % gaps_count)
            } else {
                (0, 0, 0)
            }
        }
    };

    // 4. Ubicamos a cada hijo recortando dentro de `content_rect`.
    let mut child_indices = Vec::with_capacity(n);
    for (i, child) in children.iter().enumerate() {
        let intr = intrinsic_sizes
            .get(child_preorder[i])
            .copied()
            .unwrap_or(Size::ZERO);

        let start_main = main_cursor.min(main_avail);
        let max_main_here = main_avail.saturating_sub(start_main);
        let child_main = allocated_main[i].min(max_main_here);

        let cross_rule = if is_row { child.height } else { child.width };
        let intr_cross = if is_row { intr.height } else { intr.width };
        let is_separator = matches!(child.kind, WidgetKind::Separator { .. });

        let child_cross = match cross_rule {
            Length::Fixed(px) => scale_logical_px(px, scale_milli).min(cross_avail),
            Length::Fill(_) => cross_avail,
            Length::Auto => {
                if cross_align == CrossAlign::Stretch || is_separator {
                    cross_avail
                } else {
                    intr_cross.min(cross_avail)
                }
            }
        };

        let start_cross = match cross_align {
            CrossAlign::Start | CrossAlign::Stretch => 0,
            CrossAlign::Center => cross_avail.saturating_sub(child_cross) / 2,
            CrossAlign::End => cross_avail.saturating_sub(child_cross),
        };

        let child_rect = if is_row {
            Rect::new(
                content_rect.x.saturating_add_unsigned(start_main),
                content_rect.y.saturating_add_unsigned(start_cross),
                child_main,
                child_cross,
            )
        } else {
            Rect::new(
                content_rect.x.saturating_add_unsigned(start_cross),
                content_rect.y.saturating_add_unsigned(start_main),
                child_cross,
                child_main,
            )
        };

        let child_idx =
            place_node_recursive(child, child_rect, scale_milli, intrinsic_sizes, cursor, out);
        child_indices.push(child_idx);

        let mut step_gap = gap_phys.saturating_add(extra_gap_base);
        if extra_gap_rem > 0 {
            step_gap = step_gap.saturating_add(1);
            extra_gap_rem -= 1;
        }
        main_cursor = main_cursor
            .saturating_add(allocated_main[i])
            .saturating_add(step_gap);
    }

    child_indices
}

// ============================================================================
// Capa 3: Hit-testing, estado interactivo y ruteo de eventos
// ============================================================================

/// Botón del mouse asociado a un evento de presión o soltado.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PointerButton {
    /// Botón principal (izquierdo): único botón que interactúa con los widgets.
    #[default]
    Left,
    /// Botón secundario (derecho).
    Right,
    /// Botón central (rueda).
    Middle,
    /// Cualquier otro botón adicional del mouse (`XButton1`, `XButton2`, etc.).
    Other(u8),
}

/// Entrada del puntero entregada a [`UiState::handle`], en coordenadas físicas de píxeles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerInput {
    /// El puntero se movió a `(x, y)` dentro de la superficie.
    Move {
        /// Coordenada X física en píxeles.
        x: i32,
        /// Coordenada Y física en píxeles.
        y: i32,
    },
    /// El puntero salió de la superficie (`WM_MOUSELEAVE`).
    Leave,
    /// Se presionó un botón del mouse en `(x, y)`.
    Down {
        /// Coordenada X física en píxeles.
        x: i32,
        /// Coordenada Y física en píxeles.
        y: i32,
        /// Botón presionado. Solo [`PointerButton::Left`] interactúa con los widgets.
        button: PointerButton,
    },
    /// Se soltó un botón del mouse en `(x, y)`.
    Up {
        /// Coordenada X física en píxeles.
        x: i32,
        /// Coordenada Y física en píxeles.
        y: i32,
        /// Botón soltado. Solo [`PointerButton::Left`] interactúa con los widgets.
        button: PointerButton,
    },
    /// Giro de la rueda del mouse en `(x, y)`. Se ignora sin modificar el estado.
    Wheel {
        /// Coordenada X física en píxeles.
        x: i32,
        /// Coordenada Y física en píxeles.
        y: i32,
        /// Desplazamiento horizontal de la rueda.
        delta_x: i16,
        /// Desplazamiento vertical de la rueda.
        delta_y: i16,
    },
}

impl PointerInput {
    /// Construye un evento [`PointerInput::Move`] en `(x, y)`.
    #[must_use]
    pub const fn move_to(x: i32, y: i32) -> Self {
        Self::Move { x, y }
    }

    /// Construye un evento [`PointerInput::Down`] del botón izquierdo en `(x, y)`.
    #[must_use]
    pub const fn left_down(x: i32, y: i32) -> Self {
        Self::Down {
            x,
            y,
            button: PointerButton::Left,
        }
    }

    /// Construye un evento [`PointerInput::Up`] del botón izquierdo en `(x, y)`.
    #[must_use]
    pub const fn left_up(x: i32, y: i32) -> Self {
        Self::Up {
            x,
            y,
            button: PointerButton::Left,
        }
    }

    /// Construye un evento [`PointerInput::Down`] con un botón explícito en `(x, y)`.
    #[must_use]
    pub const fn button_down(x: i32, y: i32, button: PointerButton) -> Self {
        Self::Down { x, y, button }
    }

    /// Construye un evento [`PointerInput::Up`] con un botón explícito en `(x, y)`.
    #[must_use]
    pub const fn button_up(x: i32, y: i32, button: PointerButton) -> Self {
        Self::Up { x, y, button }
    }

    /// Construye un evento [`PointerInput::Wheel`] en `(x, y)`.
    #[must_use]
    pub const fn wheel(x: i32, y: i32, delta_x: i16, delta_y: i16) -> Self {
        Self::Wheel {
            x,
            y,
            delta_x,
            delta_y,
        }
    }
}

/// Evento semántico emitido cuando el usuario interactúa con un widget.
///
/// El toolkit nunca muta el árbol [`Node`]: devuelve estos eventos para que la
/// aplicación actualice su estado y vuelva a invocar [`compute_layout`] y [`UiState::sync`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiEvent {
    /// Se completó un clic izquierdo (presionar y soltar dentro del mismo `Button` o `IconButton`).
    Clicked(WidgetId),
    /// Un `Toggle` pidió cambiar su estado booleano a `checked` tras soltarse dentro de él.
    Toggled {
        /// Identificador del `Toggle`.
        id: WidgetId,
        /// Nuevo valor booleano solicitado (`!checked_actual`).
        checked: bool,
    },
    /// Un `Slider` pidió actualizar su valor normalizado (al presionar sobre la pista o arrastrar).
    SliderChanged {
        /// Identificador del `Slider`.
        id: WidgetId,
        /// Nuevo valor normalizado en milésimas (`0..=1000`).
        value: Permille,
    },
}

/// Resultado de procesar una entrada del puntero con [`UiState::handle`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InteractionOutcome {
    /// `true` únicamente si cambió algún estado visual (`hovered`, `pressed` visual o `dragging_slider`)
    /// o se emitió un evento de arrastre/interacción, indicando que hay que redibujar una vez.
    pub needs_redraw: bool,
    /// Eventos de alto nivel producidos por la interacción (0 o 1 en la práctica).
    pub events: Vec<UiEvent>,
}

impl InteractionOutcome {
    /// Resultado vacío sin cambios visuales ni eventos (`needs_redraw = false`).
    pub const NONE: Self = Self {
        needs_redraw: false,
        events: Vec::new(),
    };
}

/// Determina qué widget interactivo habilitado se encuentra debajo del punto físico `(x, y)`.
///
/// # Reglas de hit-testing
///
/// - Solo son objetivos los widgets interactivos habilitados (`enabled == true` y
///   `id != WidgetId::NONE`): [`WidgetKind::Button`] (incluyendo `Node::icon_button`),
///   [`WidgetKind::Toggle`] y [`WidgetKind::Slider`].
/// - `Label`, `Icon`, `Spacer`, `Separator` (`Divider`), `Progress` (`ProgressBar`),
///   los contenedores (`Row`, `Column`, `Box`) y cualquier widget con `enabled == false`
///   nunca capturan el puntero.
/// - Si dos hermanos o subárboles se solapan, gana el último hijo (el que se dibuja encima).
/// - Cada hijo queda recortado por el `content_rect` de todos sus contenedores ancestros:
///   ningún hijo responde fuera de la región visible de su padre.
/// - Bordes del rectángulo: inclusivo a izquierda (`x`) y arriba (`y`), exclusivo a derecha
///   (`x + width`) y abajo (`y + height`).
#[must_use]
pub fn hit_test(tree: &Node, layout: &LayoutTree, x: i32, y: i32) -> Option<WidgetId> {
    let root_lnode = layout.nodes.first()?;
    let surface_clip = Rect::from_size(layout.available).intersect(root_lnode.rect);
    hit_test_recursive(tree, 0, surface_clip, layout, x, y, 1)
}

fn hit_test_recursive(
    node: &Node,
    layout_idx: usize,
    clip: Rect,
    layout: &LayoutTree,
    x: i32,
    y: i32,
    depth: usize,
) -> Option<WidgetId> {
    if depth > MAX_TREE_DEPTH {
        return None;
    }
    let lnode = layout.nodes.get(layout_idx)?;
    let visible = clip.intersect(lnode.rect);
    if !visible.contains(x, y) {
        return None;
    }

    match (&node.kind, &lnode.kind) {
        (
            WidgetKind::Row { children, .. } | WidgetKind::Column { children, .. },
            LayoutKind::Container {
                children: layout_children,
            },
        ) => {
            let content_clip = visible.intersect(lnode.content_rect);
            if !content_clip.contains(x, y) {
                return None;
            }
            for (child_node, &child_idx) in children.iter().zip(layout_children.iter()).rev() {
                if let Some(hit) =
                    hit_test_recursive(child_node, child_idx, content_clip, layout, x, y, depth + 1)
                {
                    return Some(hit);
                }
            }
            None
        }
        (WidgetKind::Button { enabled: true, .. }, LayoutKind::Button { enabled: true, .. })
        | (WidgetKind::Toggle { enabled: true, .. }, LayoutKind::Toggle { enabled: true, .. })
        | (WidgetKind::Slider { enabled: true, .. }, LayoutKind::Slider { enabled: true, .. }) => {
            if !node.id.is_none() && node.id == lnode.id {
                Some(node.id)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Información resuelta de un widget interactivo habilitado dentro de `(tree, layout)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InteractiveTarget {
    Button,
    Toggle { checked: bool },
    Slider { track_rect: Rect },
}

/// Busca un widget interactivo habilitado por `id` verificando tanto `tree` como `layout`.
fn find_interactive_target(
    tree: &Node,
    layout: &LayoutTree,
    id: WidgetId,
) -> Option<InteractiveTarget> {
    if id.is_none() {
        return None;
    }
    find_interactive_recursive(tree, 0, layout, id, 1)
}

fn find_interactive_recursive(
    node: &Node,
    layout_idx: usize,
    layout: &LayoutTree,
    target_id: WidgetId,
    depth: usize,
) -> Option<InteractiveTarget> {
    if depth > MAX_TREE_DEPTH {
        return None;
    }
    let lnode = layout.nodes.get(layout_idx)?;

    if node.id == target_id && lnode.id == target_id {
        return match (&node.kind, &lnode.kind) {
            (
                WidgetKind::Button { enabled: true, .. },
                LayoutKind::Button { enabled: true, .. },
            ) => Some(InteractiveTarget::Button),
            (
                WidgetKind::Toggle {
                    checked,
                    enabled: true,
                },
                LayoutKind::Toggle { enabled: true, .. },
            ) => Some(InteractiveTarget::Toggle { checked: *checked }),
            (
                WidgetKind::Slider { enabled: true, .. },
                LayoutKind::Slider { enabled: true, .. },
            ) => Some(InteractiveTarget::Slider {
                track_rect: lnode.content_rect,
            }),
            _ => None,
        };
    }

    if let (
        WidgetKind::Row { children, .. } | WidgetKind::Column { children, .. },
        LayoutKind::Container {
            children: layout_children,
        },
    ) = (&node.kind, &lnode.kind)
    {
        for (child_node, &child_idx) in children.iter().zip(layout_children.iter()) {
            if let Some(found) =
                find_interactive_recursive(child_node, child_idx, layout, target_id, depth + 1)
            {
                return Some(found);
            }
        }
    }

    None
}

/// Calcula el valor en [`Permille`] (`0..=1000`) correspondiente a la coordenada `x`
/// sobre la pista horizontal `track_rect`, usando aritmética entera saturante y
/// evitando cualquier división por cero cuando `track_rect.width == 0`.
#[must_use]
pub const fn slider_value_from_x(track_rect: Rect, x: i32) -> Permille {
    if track_rect.width == 0 || x <= track_rect.x {
        return Permille::ZERO;
    }
    let right = track_rect.right();
    if x >= right {
        return Permille::FULL;
    }
    let offset = x.abs_diff(track_rect.x) as u64;
    let width = track_rect.width as u64;
    let numer = offset.saturating_mul(1000).saturating_add(width / 2);
    let raw = (numer / width) as u16;
    Permille::new(raw)
}

/// Máquina de estados interactiva de `atoll-ui` (Capa 3).
///
/// Guarda por [`WidgetId`] qué widget está bajo el puntero (`hovered`), qué botón o
/// interruptor capturó una presión activa (`captured_press`) y qué deslizador se está
/// arrastrando (`dragging_slider`). No guarda referencias al árbol ni aloja memoria en el heap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UiState {
    hovered: Option<WidgetId>,
    captured_press: Option<WidgetId>,
    dragging_slider: Option<WidgetId>,
}

impl UiState {
    /// Crea un estado interactivo vacío en reposo.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            hovered: None,
            captured_press: None,
            dragging_slider: None,
        }
    }

    /// Devuelve el [`WidgetId`] que se encuentra actualmente en estado de hover.
    #[must_use]
    pub const fn hovered(&self) -> Option<WidgetId> {
        self.hovered
    }

    /// Devuelve el [`WidgetId`] que se ve presionado en este instante.
    ///
    /// Un `Button` o `Toggle` capturado solo se ve presionado mientras el puntero
    /// permanece dentro de ese mismo widget (`captured_press == hovered`). Si el
    /// puntero sale sin soltar el botón izquierdo, `pressed()` pasa a `None` pero
    /// [`UiState::captured_press`] conserva la captura hasta que se suelte.
    #[must_use]
    pub const fn pressed(&self) -> Option<WidgetId> {
        match (self.captured_press, self.hovered) {
            (Some(captured), Some(hovered)) if captured.0 == hovered.0 => Some(captured),
            _ => None,
        }
    }

    /// Devuelve el [`WidgetId`] del `Button` o `Toggle` que inició el gesto de presión actual,
    /// aunque el puntero haya salido temporalmente del widget.
    #[must_use]
    pub const fn captured_press(&self) -> Option<WidgetId> {
        self.captured_press
    }

    /// Devuelve el [`WidgetId`] del `Slider` que está siendo arrastrado actualmente.
    #[must_use]
    pub const fn dragging_slider(&self) -> Option<WidgetId> {
        self.dragging_slider
    }

    /// Indica si `id` tiene actualmente el puntero encima (`hover`).
    #[must_use]
    pub const fn is_hovered(&self, id: WidgetId) -> bool {
        if id.is_none() {
            return false;
        }
        matches!(self.hovered, Some(h) if h.0 == id.0)
    }

    /// Indica si `id` se ve actualmente presionado (`Button` o `Toggle` con captura y puntero adentro).
    #[must_use]
    pub const fn is_pressed(&self, id: WidgetId) -> bool {
        if id.is_none() {
            return false;
        }
        matches!(self.pressed(), Some(p) if p.0 == id.0)
    }

    /// Indica si `id` es un `Slider` que está siendo arrastrado en este momento.
    #[must_use]
    pub const fn is_dragging(&self, id: WidgetId) -> bool {
        if id.is_none() {
            return false;
        }
        matches!(self.dragging_slider, Some(d) if d.0 == id.0)
    }

    /// Indica si `id` está visualmente activo (presionado dentro de sus límites o arrastrándose).
    #[must_use]
    pub const fn is_active(&self, id: WidgetId) -> bool {
        self.is_pressed(id) || self.is_dragging(id)
    }

    /// Sincroniza el estado interactivo tras un cambio en el árbol o en su layout.
    ///
    /// Si algún [`WidgetId`] que estaba en `hovered`, `captured_press` o `dragging_slider`
    /// dejó de existir en `(tree, layout)` o dejó de ser un widget interactivo habilitado
    /// de su misma clase, limpia ese estado y devuelve `true` si hace falta redibujar.
    pub fn sync(&mut self, tree: &Node, layout: &LayoutTree) -> bool {
        let mut changed = false;

        if let Some(id) = self.hovered {
            if find_interactive_target(tree, layout, id).is_none() {
                self.hovered = None;
                changed = true;
            }
        }

        if let Some(id) = self.captured_press {
            let valid_press = matches!(
                find_interactive_target(tree, layout, id),
                Some(InteractiveTarget::Button | InteractiveTarget::Toggle { .. })
            );
            if !valid_press {
                self.captured_press = None;
                changed = true;
            }
        }

        if let Some(id) = self.dragging_slider {
            let valid_slider = matches!(
                find_interactive_target(tree, layout, id),
                Some(InteractiveTarget::Slider { .. })
            );
            if !valid_slider {
                self.dragging_slider = None;
                changed = true;
            }
        }

        changed
    }

    /// Procesa una entrada del puntero contra `tree` y `layout` sin mutar el árbol.
    ///
    /// Devuelve [`InteractionOutcome`] con `needs_redraw` exacto y los [`UiEvent`] emitidos.
    #[must_use]
    pub fn handle(
        &mut self,
        tree: &Node,
        layout: &LayoutTree,
        input: PointerInput,
    ) -> InteractionOutcome {
        let prev_hovered = self.hovered;
        let prev_pressed_visual = self.pressed();
        let prev_dragging = self.dragging_slider;

        // Garantizamos que ningún id interno apunte a un nodo inexistente si el árbol cambió.
        let _ = self.sync(tree, layout);

        let mut events = Vec::new();

        match input {
            PointerInput::Move { x, y } => {
                let hit = hit_test(tree, layout, x, y);
                if let Some(slider_id) = self.dragging_slider {
                    self.hovered = if hit == Some(slider_id) {
                        Some(slider_id)
                    } else {
                        None
                    };
                    if let Some(InteractiveTarget::Slider { track_rect }) =
                        find_interactive_target(tree, layout, slider_id)
                    {
                        let value = slider_value_from_x(track_rect, x);
                        events.push(UiEvent::SliderChanged {
                            id: slider_id,
                            value,
                        });
                    }
                } else if let Some(captured_id) = self.captured_press {
                    // Mientras el gesto está capturado por un Button/Toggle, solo ese widget
                    // puede recuperar el hover (y volver a verse presionado) al reingresar.
                    self.hovered = if hit == Some(captured_id) {
                        Some(captured_id)
                    } else {
                        None
                    };
                } else {
                    self.hovered = hit;
                }
            }
            PointerInput::Leave => {
                // Al salir de la superficie se limpia el hover (un botón capturado deja de
                // verse presionado, pero conserva `captured_press` y un slider conserva el arrastre).
                self.hovered = None;
            }
            PointerInput::Down {
                x,
                y,
                button: PointerButton::Left,
            } => {
                let hit = hit_test(tree, layout, x, y);
                self.hovered = hit;
                self.captured_press = None;
                self.dragging_slider = None;

                if let Some(id) = hit {
                    match find_interactive_target(tree, layout, id) {
                        Some(InteractiveTarget::Button | InteractiveTarget::Toggle { .. }) => {
                            self.captured_press = Some(id);
                        }
                        Some(InteractiveTarget::Slider { track_rect }) => {
                            self.dragging_slider = Some(id);
                            let value = slider_value_from_x(track_rect, x);
                            events.push(UiEvent::SliderChanged { id, value });
                        }
                        None => {}
                    }
                }
            }
            PointerInput::Up {
                x,
                y,
                button: PointerButton::Left,
            } => {
                let hit = hit_test(tree, layout, x, y);
                let was_captured = self.captured_press.take();
                let was_dragging = self.dragging_slider.take();
                self.hovered = hit;

                if was_dragging.is_none() {
                    if let Some(captured_id) = was_captured {
                        if hit == Some(captured_id) {
                            match find_interactive_target(tree, layout, captured_id) {
                                Some(InteractiveTarget::Button) => {
                                    events.push(UiEvent::Clicked(captured_id));
                                }
                                Some(InteractiveTarget::Toggle { checked }) => {
                                    events.push(UiEvent::Toggled {
                                        id: captured_id,
                                        checked: !checked,
                                    });
                                }
                                Some(InteractiveTarget::Slider { .. }) | None => {}
                            }
                        }
                    }
                }
            }
            PointerInput::Down {
                button: PointerButton::Right | PointerButton::Middle | PointerButton::Other(_),
                ..
            }
            | PointerInput::Up {
                button: PointerButton::Right | PointerButton::Middle | PointerButton::Other(_),
                ..
            }
            | PointerInput::Wheel { .. } => {
                // Solo el botón izquierdo interactúa; los demás botones y la rueda se ignoran.
            }
        }

        let needs_redraw = self.hovered != prev_hovered
            || self.pressed() != prev_pressed_visual
            || self.dragging_slider != prev_dragging
            || !events.is_empty();

        InteractionOutcome {
            needs_redraw,
            events,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::mem::size_of;

    use super::{
        compute_layout, find_interactive_target, hit_test, slider_value_from_x, validate_tree,
        ButtonVariant, CrossAlign, Edges, IconId, InteractionOutcome, LayoutKind, Length,
        MainAlign, Node, Permille, PointerButton, PointerInput, Rect, Size, TextMeasurer, Tone,
        TreeError, UiEvent, UiState, WidgetId, WidgetKind, MAX_LOGICAL_PX, MAX_TREE_DEPTH,
        MAX_TREE_NODES,
    };

    /// Medidor de texto falso y determinista para tests en cualquier SO:
    /// cada carácter (`chars().count()`) mide `size_px / 2` de ancho y `size_px` de alto.
    struct FakeMeasurer;

    impl TextMeasurer for FakeMeasurer {
        fn measure_line(&mut self, text: &str, size_px: u32) -> Size {
            let chars = text.chars().count() as u32;
            let char_w = (size_px / 2).max(1);
            Size::new(chars.saturating_mul(char_w), size_px)
        }
    }

    #[test]
    fn node_memory_footprint_stays_compact() {
        assert!(
            size_of::<Node>() <= 80,
            "Node ocupa {} bytes (presupuesto <= 80 bytes)",
            size_of::<Node>()
        );
    }

    #[test]
    fn empty_container_and_single_node_layout_correctly() {
        let mut measurer = FakeMeasurer;

        // Contenedor vacío (`Row` sin hijos) con Fill(1) ocupa el espacio disponible.
        let empty = Node::row(0, MainAlign::Start, CrossAlign::Start, Vec::new())
            .with_width(Length::Fill(1))
            .with_height(Length::Fill(1));
        let tree = compute_layout(&empty, Size::new(400, 40), 1.0, &mut measurer).expect("layout");
        assert_eq!(tree.nodes.len(), 1);
        assert_eq!(tree.nodes[0].rect, Rect::new(0, 0, 400, 40));

        // Un único nodo `Label` con tamaño Auto ("09:05" = 5 chars * 11 px = 55x22).
        let single = Node::label("09:05", 22, Tone::Primary);
        let single_tree =
            compute_layout(&single, Size::new(400, 40), 1.0, &mut measurer).expect("layout");
        assert_eq!(single_tree.nodes.len(), 1);
        assert_eq!(single_tree.nodes[0].rect, Rect::new(0, 0, 55, 22));
    }

    #[test]
    fn row_and_column_distribute_fixed_auto_and_fill_with_remainder_pixels() {
        let mut measurer = FakeMeasurer;

        // Row de 100 px de ancho con 3 hijos Fill(1): 100 / 3 = 33 con resto 1 -> 34, 33, 33.
        let row = Node::row(
            0,
            MainAlign::Start,
            CrossAlign::Stretch,
            vec![
                Node::spacer_fill(1).with_id(WidgetId::new(1)),
                Node::spacer_fill(1).with_id(WidgetId::new(2)),
                Node::spacer_fill(1).with_id(WidgetId::new(3)),
            ],
        )
        .with_fixed_size(100, 20);

        let layout = compute_layout(&row, Size::new(100, 20), 1.0, &mut measurer).expect("layout");
        let n1 = layout.find_by_id(WidgetId::new(1)).expect("n1");
        let n2 = layout.find_by_id(WidgetId::new(2)).expect("n2");
        let n3 = layout.find_by_id(WidgetId::new(3)).expect("n3");

        assert_eq!(n1.rect, Rect::new(0, 0, 34, 20));
        assert_eq!(n2.rect, Rect::new(34, 0, 33, 20));
        assert_eq!(n3.rect, Rect::new(67, 0, 33, 20));
        assert_eq!(
            n1.rect.width + n2.rect.width + n3.rect.width,
            100,
            "no se pierde ni sobra ningún píxel"
        );

        // Column combinando Fixed(10), Auto (Label 16 px alto) y dos Fill con pesos 1 y 2.
        // Alto total = 100, gap = 4 (3 gaps = 12). Espacio para Fill = 100 - 12 - 10 - 16 = 62.
        // Peso 1 -> 21 px (20 + 1 de resto), Peso 2 -> 41 px.
        let col = Node::column(
            4,
            MainAlign::Start,
            CrossAlign::Stretch,
            vec![
                Node::new(WidgetKind::Spacer)
                    .with_id(WidgetId::new(10))
                    .with_height(Length::Fixed(10)),
                Node::label("Hola", 16, Tone::Primary).with_id(WidgetId::new(11)),
                Node::spacer_fill(1).with_id(WidgetId::new(12)),
                Node::spacer_fill(2).with_id(WidgetId::new(13)),
            ],
        )
        .with_fixed_size(80, 100);

        let col_layout =
            compute_layout(&col, Size::new(80, 100), 1.0, &mut measurer).expect("layout");
        assert_eq!(
            col_layout.find_by_id(WidgetId::new(10)).expect("10").rect,
            Rect::new(0, 0, 80, 10)
        );
        assert_eq!(
            col_layout.find_by_id(WidgetId::new(11)).expect("11").rect,
            Rect::new(0, 14, 80, 16)
        );
        assert_eq!(
            col_layout.find_by_id(WidgetId::new(12)).expect("12").rect,
            Rect::new(0, 34, 80, 21)
        );
        assert_eq!(
            col_layout.find_by_id(WidgetId::new(13)).expect("13").rect,
            Rect::new(0, 59, 80, 41)
        );
    }

    #[test]
    fn padding_gap_and_alignments_position_children_accurately() {
        let mut measurer = FakeMeasurer;

        // Row de 200x40 con padding simétrico (10 horizontal, 4 vertical) -> content_rect = (10, 4, 180, 32).
        // Dos hijos de 40x16 con gap 20 -> ocupan 100 px.
        // Con MainAlign::Center -> offset X = (180 - 100) / 2 = 40 -> x1 = 10 + 40 = 50, x2 = 50 + 40 + 20 = 110.
        // Con CrossAlign::Center -> offset Y = (32 - 16) / 2 = 8 -> y = 4 + 8 = 12.
        let centered = Node::row(
            20,
            MainAlign::Center,
            CrossAlign::Center,
            vec![
                Node::new(WidgetKind::Spacer)
                    .with_id(WidgetId::new(1))
                    .with_fixed_size(40, 16),
                Node::new(WidgetKind::Spacer)
                    .with_id(WidgetId::new(2))
                    .with_fixed_size(40, 16),
            ],
        )
        .with_fixed_size(200, 40)
        .with_padding(Edges::symmetric(10, 4));

        let tree =
            compute_layout(&centered, Size::new(200, 40), 1.0, &mut measurer).expect("layout");
        assert_eq!(tree.nodes[0].content_rect, Rect::new(10, 4, 180, 32));
        assert_eq!(
            tree.find_by_id(WidgetId::new(1)).expect("1").rect,
            Rect::new(50, 12, 40, 16)
        );
        assert_eq!(
            tree.find_by_id(WidgetId::new(2)).expect("2").rect,
            Rect::new(110, 12, 40, 16)
        );

        // SpaceBetween + CrossAlign::End sobre el mismo contenedor:
        // x1 = 10, x2 = 10 + 180 - 40 = 150; y = 4 + (32 - 16) = 20.
        let between = Node::row(
            0,
            MainAlign::SpaceBetween,
            CrossAlign::End,
            vec![
                Node::new(WidgetKind::Spacer)
                    .with_id(WidgetId::new(1))
                    .with_fixed_size(40, 16),
                Node::new(WidgetKind::Spacer)
                    .with_id(WidgetId::new(2))
                    .with_fixed_size(40, 16),
            ],
        )
        .with_fixed_size(200, 40)
        .with_padding(Edges::symmetric(10, 4));

        let tree_between =
            compute_layout(&between, Size::new(200, 40), 1.0, &mut measurer).expect("layout");
        assert_eq!(
            tree_between.find_by_id(WidgetId::new(1)).expect("1").rect,
            Rect::new(10, 20, 40, 16)
        );
        assert_eq!(
            tree_between.find_by_id(WidgetId::new(2)).expect("2").rect,
            Rect::new(150, 20, 40, 16)
        );
    }

    #[test]
    fn zero_size_container_and_overflowing_fixed_children_are_clipped_inside_parent() {
        let mut measurer = FakeMeasurer;

        // Contenedor de tamaño 0x0: nada entra en pánico y todos los rectángulos son 0x0.
        let tree_zero = compute_layout(
            &Node::row(
                8,
                MainAlign::Start,
                CrossAlign::Center,
                vec![
                    Node::label("Texto largo", 20, Tone::Primary).with_id(WidgetId::new(1)),
                    Node::button(
                        WidgetId::new(2),
                        Some("Aceptar".to_owned()),
                        Some(IconId::Check),
                        ButtonVariant::Primary,
                        true,
                    ),
                ],
            ),
            Size::ZERO,
            1.0,
            &mut measurer,
        )
        .expect("layout en 0x0");
        for node in &tree_zero.nodes {
            assert_eq!(node.rect, Rect::ZERO);
            assert_eq!(node.content_rect, Rect::ZERO);
        }

        // Desborde de tamaños Fixed: Row de 100 px de ancho con 3 hijos Fixed de 60 px cada uno.
        // Hijo 1 -> x=0, w=60; Hijo 2 -> x=60, w=40 (recortado a 100); Hijo 3 -> x=100, w=0.
        let overflow = Node::row(
            0,
            MainAlign::Start,
            CrossAlign::Start,
            vec![
                Node::new(WidgetKind::Spacer)
                    .with_id(WidgetId::new(1))
                    .with_fixed_size(60, 50),
                Node::new(WidgetKind::Spacer)
                    .with_id(WidgetId::new(2))
                    .with_fixed_size(60, 50),
                Node::new(WidgetKind::Spacer)
                    .with_id(WidgetId::new(3))
                    .with_fixed_size(60, 50),
            ],
        )
        .with_fixed_size(100, 30);

        let tree_ov =
            compute_layout(&overflow, Size::new(100, 30), 1.0, &mut measurer).expect("layout");
        let parent_rect = tree_ov.nodes[0].rect;
        let c1 = tree_ov.find_by_id(WidgetId::new(1)).expect("1").rect;
        let c2 = tree_ov.find_by_id(WidgetId::new(2)).expect("2").rect;
        let c3 = tree_ov.find_by_id(WidgetId::new(3)).expect("3").rect;

        assert_eq!(c1, Rect::new(0, 0, 60, 30));
        assert_eq!(c2, Rect::new(60, 0, 40, 30));
        assert_eq!(c3, Rect::new(100, 0, 0, 30));
        assert!(parent_rect.contains_rect(c1));
        assert!(parent_rect.contains_rect(c2));
        assert!(parent_rect.contains_rect(c3));
    }

    #[test]
    fn tree_validation_rejects_excess_depth_excess_nodes_duplicate_ids_and_invalid_ranges() {
        // 1. Profundidad > MAX_TREE_DEPTH (32).
        let mut deep = Node::label("hoja", 14, Tone::Primary);
        for _ in 0..MAX_TREE_DEPTH {
            deep = Node::column(0, MainAlign::Start, CrossAlign::Start, vec![deep]);
        }
        assert_eq!(
            validate_tree(&deep),
            Err(TreeError::MaxDepthExceeded {
                max: MAX_TREE_DEPTH
            })
        );

        // 2. Cantidad de nodos > MAX_TREE_NODES (1024).
        let many_children: Vec<Node> = (0..MAX_TREE_NODES)
            .map(|_| Node::new(WidgetKind::Spacer))
            .collect();
        let huge = Node::row(0, MainAlign::Start, CrossAlign::Start, many_children);
        assert_eq!(
            validate_tree(&huge),
            Err(TreeError::MaxNodesExceeded {
                max: MAX_TREE_NODES
            })
        );

        // 3. Ids duplicados.
        let dup = Node::row(
            0,
            MainAlign::Start,
            CrossAlign::Start,
            vec![
                Node::toggle(WidgetId::new(7), true, true),
                Node::toggle(WidgetId::new(7), false, true),
            ],
        );
        assert_eq!(
            validate_tree(&dup),
            Err(TreeError::DuplicateWidgetId(WidgetId::new(7)))
        );

        // 4. Fill(0) inválido.
        let zero_fill = Node::new(WidgetKind::Spacer)
            .with_id(WidgetId::new(3))
            .with_width(Length::Fill(0));
        assert_eq!(
            validate_tree(&zero_fill),
            Err(TreeError::InvalidFillWeight {
                id: WidgetId::new(3)
            })
        );

        // 5. Permille fuera de rango (> 1000) y medida lógica > MAX_LOGICAL_PX.
        let bad_slider = Node::slider(WidgetId::new(9), Permille::from_raw(1500), true);
        assert_eq!(
            validate_tree(&bad_slider),
            Err(TreeError::ValueOutOfRange {
                id: WidgetId::new(9),
                field: "value",
                value: 1500,
                min: 0,
                max: 1000,
            })
        );

        let huge_px = Node::new(WidgetKind::Spacer).with_width(Length::Fixed(MAX_LOGICAL_PX + 1));
        assert!(matches!(
            validate_tree(&huge_px),
            Err(TreeError::ValueOutOfRange { field: "width", .. })
        ));
    }

    #[test]
    fn layout_is_strictly_deterministic_across_repeated_runs() {
        let mut measurer = FakeMeasurer;
        let tree_def = Node::column(
            6,
            MainAlign::Center,
            CrossAlign::Stretch,
            vec![
                Node::row(
                    8,
                    MainAlign::SpaceBetween,
                    CrossAlign::Center,
                    vec![
                        Node::icon(IconId::Play, 18, Tone::Accent).with_id(WidgetId::new(1)),
                        Node::label("Reproduciendo", 16, Tone::Primary).with_id(WidgetId::new(2)),
                        Node::toggle(WidgetId::new(3), true, true),
                    ],
                ),
                Node::separator(1),
                Node::slider(WidgetId::new(4), Permille::new(650), true)
                    .with_width(Length::Fill(1)),
                Node::progress(Permille::new(300), Tone::Accent).with_width(Length::Fill(1)),
            ],
        )
        .with_padding(Edges::all(12))
        .with_width(Length::Fill(1))
        .with_height(Length::Fill(1));

        let run1 =
            compute_layout(&tree_def, Size::new(480, 120), 1.5, &mut measurer).expect("run1");
        let run2 =
            compute_layout(&tree_def, Size::new(480, 120), 1.5, &mut measurer).expect("run2");
        assert_eq!(run1, run2);
    }

    /// Generador pseudoaleatorio determinista (Xorshift64) sin crates externos.
    struct Prng(u64);

    impl Prng {
        const fn new(seed: u64) -> Self {
            Self(if seed == 0 {
                0x9E37_79B9_7F4A_7C15
            } else {
                seed
            })
        }

        fn next_u32(&mut self) -> u32 {
            let mut x = self.0;
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            self.0 = x;
            (x >> 32) as u32
        }

        fn range_u16(&mut self, min: u16, max: u16) -> u16 {
            if min >= max {
                return min;
            }
            let span = u32::from(max - min) + 1;
            min + (self.next_u32() % span) as u16
        }

        fn next_length(&mut self) -> Length {
            match self.next_u32() % 3 {
                0 => Length::Auto,
                1 => Length::Fixed(self.range_u16(0, 300)),
                _ => Length::Fill(self.range_u16(1, 5) as u8),
            }
        }

        fn next_edges(&mut self) -> Edges {
            Edges::new(
                self.range_u16(0, 20),
                self.range_u16(0, 20),
                self.range_u16(0, 20),
                self.range_u16(0, 20),
            )
        }

        fn next_main_align(&mut self) -> MainAlign {
            match self.next_u32() % 4 {
                0 => MainAlign::Start,
                1 => MainAlign::Center,
                2 => MainAlign::End,
                _ => MainAlign::SpaceBetween,
            }
        }

        fn next_cross_align(&mut self) -> CrossAlign {
            match self.next_u32() % 4 {
                0 => CrossAlign::Start,
                1 => CrossAlign::Center,
                2 => CrossAlign::End,
                _ => CrossAlign::Stretch,
            }
        }

        fn gen_tree(&mut self, depth: usize, next_id: &mut u32) -> Node {
            let id = if (self.next_u32() & 1) == 0 {
                *next_id += 1;
                WidgetId::new(*next_id)
            } else {
                WidgetId::NONE
            };

            let kind_choice = if depth >= 5 {
                self.next_u32() % 8
            } else {
                self.next_u32() % 10
            };

            let kind = match kind_choice {
                0 => WidgetKind::Label {
                    text: "atoll".to_owned(),
                    size_px: self.range_u16(8, 32),
                    tone: Tone::Primary,
                },
                1 => WidgetKind::Icon {
                    icon: IconId::Settings,
                    size_px: self.range_u16(8, 32),
                    tone: Tone::Accent,
                },
                2 => WidgetKind::Spacer,
                3 => WidgetKind::Button {
                    label: Some("ok".to_owned()),
                    icon: Some(IconId::Check),
                    variant: ButtonVariant::Secondary,
                    enabled: (self.next_u32() & 3) != 0,
                },
                4 => WidgetKind::Toggle {
                    checked: (self.next_u32() & 1) == 0,
                    enabled: (self.next_u32() & 3) != 0,
                },
                5 => WidgetKind::Slider {
                    value: Permille::new(self.range_u16(0, 1000)),
                    enabled: (self.next_u32() & 3) != 0,
                },
                6 => WidgetKind::Progress {
                    value: Permille::new(self.range_u16(0, 1000)),
                    tone: Tone::Muted,
                },
                7 => WidgetKind::Separator {
                    thickness_px: self.range_u16(1, 4),
                },
                8 => {
                    let count = (self.next_u32() % 5) as usize;
                    let children = (0..count)
                        .map(|_| self.gen_tree(depth + 1, next_id))
                        .collect();
                    WidgetKind::Row {
                        gap: self.range_u16(0, 16),
                        main_align: self.next_main_align(),
                        cross_align: self.next_cross_align(),
                        children,
                    }
                }
                _ => {
                    let count = (self.next_u32() % 5) as usize;
                    let children = (0..count)
                        .map(|_| self.gen_tree(depth + 1, next_id))
                        .collect();
                    WidgetKind::Column {
                        gap: self.range_u16(0, 16),
                        main_align: self.next_main_align(),
                        cross_align: self.next_cross_align(),
                        children,
                    }
                }
            };

            Node {
                id,
                width: self.next_length(),
                height: self.next_length(),
                padding: self.next_edges(),
                kind,
            }
        }
    }

    #[test]
    fn random_valid_trees_never_panic_and_children_always_stay_within_parent_bounds() {
        let mut prng = Prng::new(0xA701_14A0_2026_1114);
        let mut measurer = FakeMeasurer;

        for trial in 0..200 {
            let mut next_id = 0;
            let tree = prng.gen_tree(1, &mut next_id);
            let avail = Size::new(
                u32::from(prng.range_u16(0, 800)),
                u32::from(prng.range_u16(0, 400)),
            );
            let scale = match trial % 4 {
                0 => 1.0,
                1 => 1.25,
                2 => 1.5,
                _ => 2.0,
            };

            let layout = compute_layout(&tree, avail, scale, &mut measurer)
                .expect("árbol pseudoaleatorio válido nunca falla");

            let surface = Rect::from_size(avail);
            assert!(
                surface.contains_rect(layout.nodes[0].rect),
                "trial {trial}: la raíz {:?} se salió de la superficie {:?}",
                layout.nodes[0].rect,
                surface
            );

            for (idx, node) in layout.nodes.iter().enumerate() {
                assert!(
                    node.rect.contains_rect(node.content_rect),
                    "trial {trial} nodo {idx}: content_rect {:?} fuera de rect {:?}",
                    node.content_rect,
                    node.rect
                );
                if let LayoutKind::Container { children } = &node.kind {
                    for &child_idx in children {
                        let child = &layout.nodes[child_idx];
                        assert!(
                            node.content_rect.contains_rect(child.rect),
                            "trial {trial}: hijo {child_idx} ({:?}) se salió del content_rect del padre {idx} ({:?})",
                            child.rect,
                            node.content_rect
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn hit_testing_exact_edges_and_interactive_targets() {
        let mut measurer = FakeMeasurer;

        // Árbol con los 4 tipos interactivos habilitados (Button, IconButton, Toggle, Slider),
        // más todos los tipos no interactivos (Label, Icon, Spacer, Divider, ProgressBar, Box)
        // con WidgetId distinto de cero y widgets interactivos deshabilitados.
        let tree = Node::column(
            0,
            MainAlign::Start,
            CrossAlign::Start,
            vec![
                // 1: Button en (0, 0, 50, 20)
                Node::button(
                    WidgetId::new(1),
                    Some("Btn".to_owned()),
                    None,
                    ButtonVariant::Primary,
                    true,
                )
                .with_fixed_size(50, 20),
                // 2: IconButton en (0, 20, 30, 20)
                Node::icon_button(WidgetId::new(2), IconId::Play, ButtonVariant::Ghost, true)
                    .with_fixed_size(30, 20),
                // 3: Toggle en (0, 40, 36, 20)
                Node::toggle(WidgetId::new(3), false, true).with_fixed_size(36, 20),
                // 4: Slider en (0, 60, 80, 20)
                Node::slider(WidgetId::new(4), Permille::new(500), true).with_fixed_size(80, 20),
                // 5..9: Widgets NO interactivos aunque tengan WidgetId != 0
                Node::label("Texto", 14, Tone::Primary)
                    .with_id(WidgetId::new(5))
                    .with_fixed_size(50, 20),
                Node::icon(IconId::Settings, 16, Tone::Accent)
                    .with_id(WidgetId::new(6))
                    .with_fixed_size(50, 20),
                Node::spacer_fixed(50, 20).with_id(WidgetId::new(7)),
                Node::divider(2)
                    .with_id(WidgetId::new(8))
                    .with_fixed_size(50, 20),
                Node::progress_bar(Permille::new(400), Tone::Accent)
                    .with_id(WidgetId::new(9))
                    .with_fixed_size(50, 20),
                // 10..12: Widgets interactivos pero deshabilitados (`enabled: false`)
                Node::button(
                    WidgetId::new(10),
                    Some("Off".to_owned()),
                    None,
                    ButtonVariant::Secondary,
                    false,
                )
                .with_fixed_size(50, 20),
                Node::toggle(WidgetId::new(11), true, false).with_fixed_size(50, 20),
                Node::slider(WidgetId::new(12), Permille::new(200), false).with_fixed_size(50, 20),
            ],
        )
        .with_id(WidgetId::new(99))
        .with_fixed_size(100, 300);

        let layout =
            compute_layout(&tree, Size::new(100, 300), 1.0, &mut measurer).expect("layout");

        // Bordes exactos de Button (0, 0, 50, 20):
        // Inclusivo a izquierda (0) y arriba (0); exclusivo a derecha (50) y abajo (20).
        assert_eq!(hit_test(&tree, &layout, 0, 0), Some(WidgetId::new(1)));
        assert_eq!(hit_test(&tree, &layout, 49, 0), Some(WidgetId::new(1)));
        assert_eq!(hit_test(&tree, &layout, 0, 19), Some(WidgetId::new(1)));
        assert_eq!(hit_test(&tree, &layout, 49, 19), Some(WidgetId::new(1)));
        assert_eq!(hit_test(&tree, &layout, -1, 0), None);
        assert_eq!(hit_test(&tree, &layout, 0, -1), None);
        assert_eq!(hit_test(&tree, &layout, 50, 0), None);
        assert_eq!(hit_test(&tree, &layout, 50, 19), None);

        // En y = 20 empieza el IconButton (id = 2, 0..30 x 20..40).
        assert_eq!(hit_test(&tree, &layout, 0, 20), Some(WidgetId::new(2)));
        assert_eq!(hit_test(&tree, &layout, 29, 39), Some(WidgetId::new(2)));
        assert_eq!(hit_test(&tree, &layout, 30, 20), None);

        // En y = 40 empieza el Toggle (id = 3, 0..36 x 40..60).
        assert_eq!(hit_test(&tree, &layout, 0, 40), Some(WidgetId::new(3)));
        assert_eq!(hit_test(&tree, &layout, 35, 59), Some(WidgetId::new(3)));
        assert_eq!(hit_test(&tree, &layout, 36, 40), None);

        // En y = 60 empieza el Slider (id = 4, 0..80 x 60..80).
        assert_eq!(hit_test(&tree, &layout, 0, 60), Some(WidgetId::new(4)));
        assert_eq!(hit_test(&tree, &layout, 79, 79), Some(WidgetId::new(4)));
        assert_eq!(hit_test(&tree, &layout, 80, 60), None);

        // Ninguno de los nodos no interactivos (y = 80..180), deshabilitados (y = 180..240)
        // ni el contenedor raíz (id = 99) captura jamás el puntero.
        for y in 80..300 {
            assert_eq!(
                hit_test(&tree, &layout, 10, y),
                None,
                "en y={y} ningún nodo no interactivo ni deshabilitado debe capturar el puntero"
            );
        }
    }

    #[test]
    fn hit_testing_overlap_and_clipped_children() {
        let mut measurer = FakeMeasurer;

        // 1. Solapamiento: tres hijos en un Row. Forzamos rectángulos solapados en `layout`:
        // - Hijo 1 (Button id=1): (10, 10, 60, 30) -> [10..70, 10..40)
        // - Hijo 2 (Button id=2): (40, 10, 60, 30) -> [40..100, 10..40) (se dibuja encima de 1 en 40..70)
        // - Hijo 3 (Label no interactivo id=3): (50, 10, 40, 30) -> no captura, deja pasar a 2.
        let overlap_tree = Node::row(
            0,
            MainAlign::Start,
            CrossAlign::Start,
            vec![
                Node::button(
                    WidgetId::new(1),
                    Some("A".to_owned()),
                    None,
                    ButtonVariant::Secondary,
                    true,
                )
                .with_fixed_size(60, 30),
                Node::button(
                    WidgetId::new(2),
                    Some("B".to_owned()),
                    None,
                    ButtonVariant::Primary,
                    true,
                )
                .with_fixed_size(60, 30),
                Node::label("Encima", 14, Tone::Primary)
                    .with_id(WidgetId::new(3))
                    .with_fixed_size(40, 30),
            ],
        )
        .with_fixed_size(120, 50);

        let mut overlap_layout =
            compute_layout(&overlap_tree, Size::new(120, 50), 1.0, &mut measurer).expect("layout");
        overlap_layout.nodes[1].rect = Rect::new(10, 10, 60, 30);
        overlap_layout.nodes[2].rect = Rect::new(40, 10, 60, 30);
        overlap_layout.nodes[3].rect = Rect::new(50, 10, 40, 30);

        // Zona exclusiva de 1: (25, 20) -> gana 1.
        assert_eq!(
            hit_test(&overlap_tree, &overlap_layout, 25, 20),
            Some(WidgetId::new(1))
        );
        // Zona donde 1 y 2 se solapan (40..70): gana el último hijo interactivo (2),
        // e ignora el Label 3 que está encima en (55, 20).
        assert_eq!(
            hit_test(&overlap_tree, &overlap_layout, 40, 20),
            Some(WidgetId::new(2))
        );
        assert_eq!(
            hit_test(&overlap_tree, &overlap_layout, 55, 20),
            Some(WidgetId::new(2))
        );
        assert_eq!(
            hit_test(&overlap_tree, &overlap_layout, 69, 20),
            Some(WidgetId::new(2))
        );
        // Zona exclusiva de 2: (85, 20) -> gana 2.
        assert_eq!(
            hit_test(&overlap_tree, &overlap_layout, 85, 20),
            Some(WidgetId::new(2))
        );

        // 2. Hijos recortados por el `content_rect` de su padre:
        // Contenedor de 100x40 con padding 10 en todos los lados -> content_rect = (10, 10, 80, 20).
        // Adentro hay un Button Fixed de 120x30 (que `compute_layout` recorta a (10, 10, 80, 20)).
        let clipped_tree = Node::box_container(
            Edges::all(10),
            vec![Node::button(
                WidgetId::new(7),
                Some("Ancho".to_owned()),
                None,
                ButtonVariant::Primary,
                true,
            )
            .with_fixed_size(120, 30)],
        )
        .with_fixed_size(100, 40);

        let mut clipped_layout =
            compute_layout(&clipped_tree, Size::new(100, 40), 1.0, &mut measurer).expect("layout");
        assert_eq!(
            clipped_layout.nodes[0].content_rect,
            Rect::new(10, 10, 80, 20)
        );

        // Incluso si el rectángulo del hijo en LayoutTree desbordara el `content_rect` del padre,
        // `hit_test` lo recorta contra el `content_rect` del padre (10..90 x 10..30).
        clipped_layout.nodes[1].rect = Rect::new(0, 0, 120, 40);
        assert_eq!(
            hit_test(&clipped_tree, &clipped_layout, 10, 10),
            Some(WidgetId::new(7))
        );
        assert_eq!(
            hit_test(&clipped_tree, &clipped_layout, 89, 29),
            Some(WidgetId::new(7))
        );
        // Fuera del `content_rect` del padre (en la zona de padding 0..10 o 90..100): no responde.
        assert_eq!(hit_test(&clipped_tree, &clipped_layout, 5, 15), None);
        assert_eq!(hit_test(&clipped_tree, &clipped_layout, 90, 15), None);
        assert_eq!(hit_test(&clipped_tree, &clipped_layout, 50, 5), None);
        assert_eq!(hit_test(&clipped_tree, &clipped_layout, 50, 30), None);
    }

    #[test]
    fn button_and_icon_button_click_capture_leave_reenter_and_exact_redraw() {
        let mut measurer = FakeMeasurer;
        let btn_a = WidgetId::new(1);
        let btn_b = WidgetId::new(2);

        // Row con Button A en (0, 0, 50, 28) e IconButton B en (60, 0, 28, 28).
        let tree = Node::row(
            10,
            MainAlign::Start,
            CrossAlign::Start,
            vec![
                Node::button(
                    btn_a,
                    Some("A".to_owned()),
                    None,
                    ButtonVariant::Primary,
                    true,
                )
                .with_fixed_size(50, 28),
                Node::icon_button(btn_b, IconId::Play, ButtonVariant::Ghost, true)
                    .with_fixed_size(28, 28),
            ],
        )
        .with_fixed_size(200, 40);

        let layout = compute_layout(&tree, Size::new(200, 40), 1.0, &mut measurer).expect("layout");
        let mut state = UiState::new();

        // 1. Moverse por zona vacía (150, 10) -> no cambia nada, needs_redraw = false.
        let out = state.handle(&tree, &layout, PointerInput::move_to(150, 10));
        assert!(!out.needs_redraw);
        assert!(out.events.is_empty());
        assert_eq!(state.hovered(), None);

        // 2. Entrar a Button A (10, 10) -> cambia hover, needs_redraw = true.
        let out = state.handle(&tree, &layout, PointerInput::move_to(10, 10));
        assert!(out.needs_redraw);
        assert_eq!(state.hovered(), Some(btn_a));
        assert!(state.is_hovered(btn_a));
        assert!(!state.is_pressed(btn_a));

        // 3. Moverse dentro del mismo Button A (20, 15) -> needs_redraw = false.
        let out = state.handle(&tree, &layout, PointerInput::move_to(20, 15));
        assert!(!out.needs_redraw);
        assert!(out.events.is_empty());

        // 4. Presionar botón izquierdo adentro de Button A -> pasa a verse presionado, needs_redraw = true.
        let out = state.handle(&tree, &layout, PointerInput::left_down(20, 15));
        assert!(out.needs_redraw);
        assert!(out.events.is_empty());
        assert_eq!(state.pressed(), Some(btn_a));
        assert_eq!(state.captured_press(), Some(btn_a));
        assert!(state.is_pressed(btn_a));
        assert!(state.is_active(btn_a));

        // 5. Moverse adentro mientras está presionado -> needs_redraw = false.
        let out = state.handle(&tree, &layout, PointerInput::move_to(25, 15));
        assert!(!out.needs_redraw);
        assert!(state.is_pressed(btn_a));

        // 6. Salir de Button A mientras sigue presionado -> deja de verse presionado,
        // pero el gesto sigue capturado (`captured_press == Some(btn_a)`). needs_redraw = true.
        let out = state.handle(&tree, &layout, PointerInput::move_to(150, 15));
        assert!(out.needs_redraw);
        assert!(out.events.is_empty());
        assert!(!state.is_pressed(btn_a));
        assert_eq!(state.pressed(), None);
        assert_eq!(state.hovered(), None);
        assert_eq!(state.captured_press(), Some(btn_a));

        // 7. Moverse afuera (incluso pasando por encima de IconButton B en x=70) mientras
        // el gesto sigue capturado por A -> no cambia ningún estado visual, needs_redraw = false.
        let out = state.handle(&tree, &layout, PointerInput::move_to(70, 15));
        assert!(!out.needs_redraw);
        assert_eq!(state.hovered(), None);
        assert!(!state.is_pressed(btn_a));

        // 8. Volver a entrar a Button A sin haber soltado -> vuelve a verse presionado, needs_redraw = true.
        let out = state.handle(&tree, &layout, PointerInput::move_to(30, 15));
        assert!(out.needs_redraw);
        assert!(state.is_pressed(btn_a));
        assert_eq!(state.pressed(), Some(btn_a));
        assert_eq!(state.hovered(), Some(btn_a));

        // 9. Soltar adentro de Button A -> emite Clicked(btn_a), limpia presión, needs_redraw = true.
        let out = state.handle(&tree, &layout, PointerInput::left_up(30, 15));
        assert!(out.needs_redraw);
        assert_eq!(out.events, vec![UiEvent::Clicked(btn_a)]);
        assert!(!state.is_pressed(btn_a));
        assert_eq!(state.captured_press(), None);
        assert_eq!(state.hovered(), Some(btn_a));

        // 10. Presionar adentro de IconButton B (65, 10), salir con Leave y soltar afuera:
        // al soltar afuera no hay clic y como ya había dejado de verse presionado al salir,
        // soltar en vacío no pide redibujar.
        let out = state.handle(&tree, &layout, PointerInput::left_down(65, 10));
        assert!(out.needs_redraw);
        assert!(state.is_pressed(btn_b));

        let out = state.handle(&tree, &layout, PointerInput::Leave);
        assert!(out.needs_redraw);
        assert!(!state.is_pressed(btn_b));
        assert_eq!(state.captured_press(), Some(btn_b));

        let out = state.handle(&tree, &layout, PointerInput::left_up(180, 10));
        assert!(!out.needs_redraw);
        assert!(out.events.is_empty());
        assert_eq!(state.captured_press(), None);
    }

    #[test]
    fn toggle_interaction_emits_event_without_mutating_tree() {
        let mut measurer = FakeMeasurer;
        let toggle_id = WidgetId::new(10);

        let tree_off = Node::toggle(toggle_id, false, true).with_fixed_size(36, 20);
        let layout_off =
            compute_layout(&tree_off, Size::new(100, 40), 1.0, &mut measurer).expect("layout");
        let mut state = UiState::new();

        // Presionar sobre el Toggle -> captura sin emitir evento todavía.
        let out = state.handle(&tree_off, &layout_off, PointerInput::left_down(10, 10));
        assert!(out.needs_redraw);
        assert!(out.events.is_empty());
        assert!(state.is_pressed(toggle_id));

        // Salir y volver a entrar conserva la captura.
        let out = state.handle(&tree_off, &layout_off, PointerInput::move_to(80, 10));
        assert!(out.needs_redraw);
        assert!(!state.is_pressed(toggle_id));

        let out = state.handle(&tree_off, &layout_off, PointerInput::move_to(15, 10));
        assert!(out.needs_redraw);
        assert!(state.is_pressed(toggle_id));

        // Soltar adentro -> emite Toggled { id, checked: true } sin mutar `tree_off`.
        let out = state.handle(&tree_off, &layout_off, PointerInput::left_up(15, 10));
        assert!(out.needs_redraw);
        assert_eq!(
            out.events,
            vec![UiEvent::Toggled {
                id: toggle_id,
                checked: true,
            }]
        );
        assert_eq!(
            tree_off.kind,
            WidgetKind::Toggle {
                checked: false,
                enabled: true,
            },
            "el toolkit nunca modifica el árbol de la aplicación"
        );

        // La aplicación actualiza su árbol a `checked: true`, recalcula layout y llama a `sync`.
        let tree_on = Node::toggle(toggle_id, true, true).with_fixed_size(36, 20);
        let layout_on =
            compute_layout(&tree_on, Size::new(100, 40), 1.0, &mut measurer).expect("layout");
        assert!(!state.sync(&tree_on, &layout_on));

        // Un nuevo clic sobre `tree_on` pide cambiar a `false`.
        let _ = state.handle(&tree_on, &layout_on, PointerInput::left_down(15, 10));
        let out = state.handle(&tree_on, &layout_on, PointerInput::left_up(15, 10));
        assert_eq!(
            out.events,
            vec![UiEvent::Toggled {
                id: toggle_id,
                checked: false,
            }]
        );

        // Presionar adentro y soltar afuera no emite Toggled.
        let _ = state.handle(&tree_on, &layout_on, PointerInput::left_down(15, 10));
        let out = state.handle(&tree_on, &layout_on, PointerInput::left_up(90, 10));
        assert!(out.events.is_empty());
    }

    #[test]
    fn slider_drag_clamps_out_of_bounds_and_handles_zero_width_track() {
        let mut measurer = FakeMeasurer;
        let slider_id = WidgetId::new(20);

        // Slider en (10, 5, 100, 20) dentro de un contenedor con padding.
        let tree = Node::box_container(
            Edges::new(5, 10, 5, 10),
            vec![Node::slider(slider_id, Permille::new(0), true).with_fixed_size(100, 20)],
        )
        .with_fixed_size(120, 30);

        let layout = compute_layout(&tree, Size::new(120, 30), 1.0, &mut measurer).expect("layout");
        let mut state = UiState::new();

        // 1. Presionar en x = 35 (25 px desde el inicio de la pista en x = 10 -> 250 permille).
        let out = state.handle(&tree, &layout, PointerInput::left_down(35, 15));
        assert!(out.needs_redraw);
        assert!(state.is_dragging(slider_id));
        assert!(state.is_active(slider_id));
        assert_eq!(
            out.events,
            vec![UiEvent::SliderChanged {
                id: slider_id,
                value: Permille::new(250),
            }]
        );

        // 2. Arrastrar a x = 74 (64 px desde x = 10 -> 640 permille), incluso saliendo en Y (y = 200).
        let out = state.handle(&tree, &layout, PointerInput::move_to(74, 200));
        assert!(out.needs_redraw);
        assert!(state.is_dragging(slider_id));
        assert_eq!(
            out.events,
            vec![UiEvent::SliderChanged {
                id: slider_id,
                value: Permille::new(640),
            }]
        );

        // 3. Arrastrar mucho más a la izquierda de la pista (x = -500) -> acotado a 0 permille.
        let out = state.handle(&tree, &layout, PointerInput::move_to(-500, -50));
        assert!(out.needs_redraw);
        assert!(state.is_dragging(slider_id));
        assert_eq!(
            out.events,
            vec![UiEvent::SliderChanged {
                id: slider_id,
                value: Permille::ZERO,
            }]
        );

        // 4. Arrastrar mucho más a la derecha de la pista (x = 5000) -> acotado a 1000 permille.
        let out = state.handle(&tree, &layout, PointerInput::move_to(5000, 500));
        assert!(out.needs_redraw);
        assert!(state.is_dragging(slider_id));
        assert_eq!(
            out.events,
            vec![UiEvent::SliderChanged {
                id: slider_id,
                value: Permille::FULL,
            }]
        );

        // 5. Soltar termina el arrastre.
        let out = state.handle(&tree, &layout, PointerInput::left_up(5000, 500));
        assert!(out.needs_redraw);
        assert!(!state.is_dragging(slider_id));
        assert!(out.events.is_empty());

        // Moverse después de soltar ya no emite SliderChanged.
        let out = state.handle(&tree, &layout, PointerInput::move_to(50, 15));
        assert!(out.events.is_empty());

        // 6. Pista de ancho cero (`track_rect.width == 0`): no divide por cero nunca.
        assert_eq!(
            slider_value_from_x(Rect::new(10, 0, 0, 20), 50),
            Permille::ZERO
        );
        let zero_track_tree = Node::slider(slider_id, Permille::new(500), true)
            .with_fixed_size(20, 20)
            .with_padding(Edges::symmetric(10, 0));
        let zero_track_layout =
            compute_layout(&zero_track_tree, Size::new(20, 20), 1.0, &mut measurer)
                .expect("layout");
        assert_eq!(zero_track_layout.nodes[0].content_rect.width, 0);

        let out = state.handle(
            &zero_track_tree,
            &zero_track_layout,
            PointerInput::left_down(5, 10),
        );
        assert_eq!(
            out.events,
            vec![UiEvent::SliderChanged {
                id: slider_id,
                value: Permille::ZERO,
            }]
        );
    }

    #[test]
    fn non_left_buttons_and_wheel_are_ignored_in_all_states() {
        let mut measurer = FakeMeasurer;
        let btn_id = WidgetId::new(1);
        let tree = Node::button(
            btn_id,
            Some("Test".to_owned()),
            None,
            ButtonVariant::Primary,
            true,
        )
        .with_fixed_size(60, 28);
        let layout = compute_layout(&tree, Size::new(100, 40), 1.0, &mut measurer).expect("layout");
        let mut state = UiState::new();

        for button in [
            PointerButton::Right,
            PointerButton::Middle,
            PointerButton::Other(4),
        ] {
            let down = state.handle(&tree, &layout, PointerInput::button_down(10, 10, button));
            assert_eq!(down, InteractionOutcome::NONE);
            assert_eq!(state, UiState::new());

            let up = state.handle(&tree, &layout, PointerInput::button_up(10, 10, button));
            assert_eq!(up, InteractionOutcome::NONE);
            assert_eq!(state, UiState::new());
        }

        let wheel = state.handle(&tree, &layout, PointerInput::wheel(10, 10, 0, 120));
        assert_eq!(wheel, InteractionOutcome::NONE);
        assert_eq!(state, UiState::new());

        // Incluso con el botón izquierdo presionado, clic derecho/central o rueda no alteran la captura.
        let _ = state.handle(&tree, &layout, PointerInput::left_down(10, 10));
        let snapshot = state;
        assert_eq!(
            state.handle(
                &tree,
                &layout,
                PointerInput::button_down(10, 10, PointerButton::Right)
            ),
            InteractionOutcome::NONE
        );
        assert_eq!(
            state.handle(
                &tree,
                &layout,
                PointerInput::button_up(10, 10, PointerButton::Right)
            ),
            InteractionOutcome::NONE
        );
        assert_eq!(
            state.handle(&tree, &layout, PointerInput::wheel(10, 10, 0, -120)),
            InteractionOutcome::NONE
        );
        assert_eq!(state, snapshot);
    }

    #[test]
    fn sync_cleans_up_removed_or_disabled_widgets_and_reports_redraw() {
        let mut measurer = FakeMeasurer;
        let btn_id = WidgetId::new(1);
        let slider_id = WidgetId::new(2);

        let tree_initial = Node::row(
            8,
            MainAlign::Start,
            CrossAlign::Start,
            vec![
                Node::button(
                    btn_id,
                    Some("Btn".to_owned()),
                    None,
                    ButtonVariant::Primary,
                    true,
                )
                .with_fixed_size(50, 28),
                Node::slider(slider_id, Permille::new(300), true).with_fixed_size(80, 20),
            ],
        )
        .with_fixed_size(200, 40);
        let layout_initial =
            compute_layout(&tree_initial, Size::new(200, 40), 1.0, &mut measurer).expect("layout");

        // Caso 1: el botón está presionado y desaparece del árbol.
        let mut state = UiState::new();
        let _ = state.handle(
            &tree_initial,
            &layout_initial,
            PointerInput::left_down(10, 10),
        );
        assert!(state.is_pressed(btn_id));

        let tree_without_btn = Node::row(
            8,
            MainAlign::Start,
            CrossAlign::Start,
            vec![Node::slider(slider_id, Permille::new(300), true).with_fixed_size(80, 20)],
        )
        .with_fixed_size(200, 40);
        let layout_without_btn =
            compute_layout(&tree_without_btn, Size::new(200, 40), 1.0, &mut measurer)
                .expect("layout");

        assert!(
            state.sync(&tree_without_btn, &layout_without_btn),
            "sync debe devolver true al limpiar el widget presionado que desapareció"
        );
        assert_eq!(state.hovered(), None);
        assert_eq!(state.pressed(), None);
        assert_eq!(state.captured_press(), None);
        assert!(
            !state.sync(&tree_without_btn, &layout_without_btn),
            "una segunda llamada a sync sin cambios devuelve false"
        );

        // Caso 2: el slider se está arrastrando y pasa a `enabled: false`.
        let _ = state.handle(
            &tree_initial,
            &layout_initial,
            PointerInput::left_down(70, 10),
        );
        assert!(state.is_dragging(slider_id));

        let tree_disabled_slider = Node::row(
            8,
            MainAlign::Start,
            CrossAlign::Start,
            vec![
                Node::button(
                    btn_id,
                    Some("Btn".to_owned()),
                    None,
                    ButtonVariant::Primary,
                    true,
                )
                .with_fixed_size(50, 28),
                Node::slider(slider_id, Permille::new(300), false).with_fixed_size(80, 20),
            ],
        )
        .with_fixed_size(200, 40);
        let layout_disabled_slider = compute_layout(
            &tree_disabled_slider,
            Size::new(200, 40),
            1.0,
            &mut measurer,
        )
        .expect("layout");

        assert!(state.sync(&tree_disabled_slider, &layout_disabled_slider));
        assert!(!state.is_dragging(slider_id));
        assert_eq!(state.hovered(), None);
    }

    #[test]
    fn interaction_is_deterministic_and_random_sequences_preserve_invariants() {
        let mut measurer = FakeMeasurer;
        let mut prng = Prng::new(0xA701_14B0_2026_1114);

        for trial in 0..150 {
            let mut next_id = 0;
            let tree = prng.gen_tree(1, &mut next_id);
            let avail = Size::new(
                u32::from(prng.range_u16(0, 400)),
                u32::from(prng.range_u16(0, 200)),
            );
            let layout = compute_layout(&tree, avail, 1.0, &mut measurer).expect("layout válido");

            let mut state_a = UiState::new();
            let mut state_b = UiState::new();

            for step in 0..60 {
                let x = i32::from(prng.range_u16(0, 450)) - 25;
                let y = i32::from(prng.range_u16(0, 250)) - 25;
                let input = match prng.next_u32() % 7 {
                    0 | 1 => PointerInput::move_to(x, y),
                    2 => PointerInput::left_down(x, y),
                    3 => PointerInput::left_up(x, y),
                    4 => PointerInput::Leave,
                    5 => PointerInput::button_down(x, y, PointerButton::Right),
                    _ => PointerInput::wheel(x, y, 0, 120),
                };

                let out_a = state_a.handle(&tree, &layout, input);
                let out_b = state_b.handle(&tree, &layout, input);

                // Determinismo exacto entre dos corridas idénticas.
                assert_eq!(
                    state_a, state_b,
                    "trial {trial} step {step}: UiState difiere"
                );
                assert_eq!(
                    out_a, out_b,
                    "trial {trial} step {step}: InteractionOutcome difiere"
                );

                // Invariante: todo WidgetId activo en `UiState` existe y es un widget interactivo habilitado.
                for active_id in [
                    state_a.hovered(),
                    state_a.pressed(),
                    state_a.captured_press(),
                    state_a.dragging_slider(),
                ]
                .into_iter()
                .flatten()
                {
                    assert!(
                        !active_id.is_none(),
                        "trial {trial} step {step}: WidgetId::NONE nunca puede quedar guardado"
                    );
                    assert!(
                        find_interactive_target(&tree, &layout, active_id).is_some(),
                        "trial {trial} step {step}: el id {:?} en UiState no existe o no es interactivo habilitado",
                        active_id
                    );
                }
            }

            // Si el árbol cambia a otro árbol aleatorio y llamamos a `sync`, los ids siguen siendo válidos.
            let mut next_id_2 = 0;
            let tree_2 = prng.gen_tree(1, &mut next_id_2);
            let layout_2 =
                compute_layout(&tree_2, avail, 1.0, &mut measurer).expect("layout_2 válido");
            let _ = state_a.sync(&tree_2, &layout_2);
            for active_id in [
                state_a.hovered(),
                state_a.pressed(),
                state_a.captured_press(),
                state_a.dragging_slider(),
            ]
            .into_iter()
            .flatten()
            {
                assert!(
                    find_interactive_target(&tree_2, &layout_2, active_id).is_some(),
                    "trial {trial}: tras sync quedó un id inválido {:?}",
                    active_id
                );
            }
        }
    }
}
