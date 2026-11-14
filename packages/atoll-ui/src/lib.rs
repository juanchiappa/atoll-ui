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

    /// Crea un [`WidgetKind::Separator`] con espesor lógico `thickness_px`.
    #[must_use]
    pub const fn separator(thickness_px: u16) -> Self {
        Self::new(WidgetKind::Separator { thickness_px })
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

#[cfg(test)]
mod tests {
    use std::mem::size_of;

    use super::{
        compute_layout, validate_tree, ButtonVariant, CrossAlign, Edges, IconId, LayoutKind,
        Length, MainAlign, Node, Permille, Rect, Size, TextMeasurer, Tone, TreeError, WidgetId,
        WidgetKind, MAX_LOGICAL_PX, MAX_TREE_DEPTH, MAX_TREE_NODES,
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
                    enabled: true,
                },
                4 => WidgetKind::Toggle {
                    checked: (self.next_u32() & 1) == 0,
                    enabled: true,
                },
                5 => WidgetKind::Slider {
                    value: Permille::new(self.range_u16(0, 1000)),
                    enabled: true,
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
}
