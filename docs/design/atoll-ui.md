# Diseño de `atoll-ui` (Ítem 14 — Mini toolkit de widgets)

- **Estado:** Propuesta para revisión antes de implementar código.
- **Crate destino:** `packages/atoll-ui`
- **Dependencias internas:** `atoll-render` y `atoll-config`.
- **Dependencias externas:** **ninguna (0 crates externos).**

---

## 1. Objetivo y alcance

`atoll-ui` es el mini toolkit declarativo de Atoll para construir y dibujar interfaces livianas sin webview ni frameworks pesados. Se usará en tres lugares:

1. **Contenido de la isla (`atoll-core`):** reloj, fecha y vistas expandidas al hacer hover.
2. **Plugins por IPC (`atoll-ipc` / ítem 15):** los plugins (música, dock, Git, IA, etc.) corren en procesos separados y envían un árbol de widgets serializado para que el Core calcule su layout, procese eventos del puntero y lo dibuje en la ventana de la isla.
3. **Menú de configuración (`atoll-settings` / ítem 17):** ejecutable independiente que edita `%APPDATA%\Atoll\config.ini`.

### Restricciones innegociables
- **Cero dependencias externas nuevas:** solo `std`, `atoll-render` y `atoll-config`.
- **Datos planos serializables sin `serde`:** el árbol de widgets (`Node`) es `Clone + PartialEq + Eq`, sin closures, sin `dyn Trait`, sin punteros ni referencias con lifetimes, de modo que en el ítem 15 pueda codificarse y decodificarse trivialmente en binario o texto propio.
- **Arquitectura en 4 capas puras:**
  1. **Árbol de widgets (`tree`):** estructuras de datos planas.
  2. **Layout (`layout`):** función pura `(Node, Size, scale, &mut TextMeasurer) -> LayoutTree`.
  3. **Hit-testing y estado interactivo (`interaction`):** máquina de estados pura `(UiState, &LayoutTree, PointerInput) -> InteractionOutcome`.
  4. **Pintado (`paint`):** dibujo sobre un `atoll-render::Frame` a partir de `LayoutTree`, `UiState` y `ThemePalette`.
  Las capas 1, 2 y 3 no tocan el sistema operativo ni `tiny-skia` y se prueban al 100 % en Linux sin ventana; la capa 4 se prueba con tests de píxeles en memoria en Linux.
- **Cero polling, cero render loop:** solo se recalcula layout o se redibuja cuando cambia el árbol (`new_tree != old_tree`), cambia el tamaño/escala de la superficie, o un evento del puntero cambia el estado visual (`needs_redraw == true`).

---

## 2. Identificación de widgets (`WidgetId`)

Cada widget interactivo lleva un identificador explícito, estable entre cuadros y pequeño:

```rust
/// Identificador estable de un widget dentro de un árbol.
///
/// Es un entero de 32 bits sin signo asignado por quien construye el árbol
/// (el Core, `atoll-settings` o un plugin). El valor `0` se reserva para
/// nodos decorativos sin interacción (`WidgetId::NONE`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WidgetId(pub u32);

impl WidgetId {
    /// Identificador nulo para widgets que no reciben interacción.
    pub const NONE: Self = Self(0);

    /// Crea un identificador a partir de un `u32` distinto de cero.
    #[must_use]
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }
}
```

### Por qué `WidgetId(u32)` en lugar de strings o rutas de índices
- **4 bytes y `Copy`:** no aloja en el heap ni fragmenta memoria.
- **Trivial de serializar por IPC:** 4 bytes en little-endian cuando se implemente `atoll-ipc`.
- **Estable ante cambios de layout:** si un plugin agrega o esconde una fila arriba de un botón, el `WidgetId(42)` del botón sigue siendo `42`, por lo que no se pierde el estado de hover o arrastre activo.
- **Aislamiento entre plugins (ítem 15):** el host de plugins en `atoll-core` podrá combinar el `plugin_id: u16` en los 16 bits altos y el id local del plugin en los 16 bits bajos sin cambiar el tipo.

---

## 3. Árbol de widgets y tipos principales (Capa 1)

Todas las medidas dentro del árbol (`font_size`, `padding`, `gap`, `width`, `height`, `thickness`) se expresan en **píxeles lógicos a 96 DPI** (`u16` o `i32` / milésimas fijas para preservar `Eq`), y se multiplican por `scale: f32` en la etapa de layout y pintado.

> **Nota sobre `Eq` en el árbol:** para que `Node` implemente `PartialEq + Eq` sin lidiar con `f32::NAN`, los valores normalizados de `Slider` y `Progress` se representan en **milésimas enteras `0..=1000` (`Permille(u16)`)**, donde `0 = 0,0 %`, `500 = 50,0 %` y `1000 = 100,0 %`. Eso da resolución de `0,1 %` (más fina que cualquier slider en píxeles de la isla o de `atoll-settings`), ocupa 2 bytes y es exacto al comparar si el árbol cambió.

```rust
/// Proporción normalizada entre 0 y 1000 (`0` = 0 %, `1000` = 100 %).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Permille(u16);

impl Permille {
    /// Mínimo (0 %).
    pub const ZERO: Self = Self(0);
    /// Máximo (100 %).
    pub const FULL: Self = Self(1000);

    /// Crea un valor acotado a `0..=1000`.
    #[must_use]
    pub fn new(value: u16) -> Self {
        Self(value.min(1000))
    }

    /// Convierte a fracción `0.0..=1.0` para interpolar en layout o pintado.
    #[must_use]
    pub fn as_ratio(self) -> f32 {
        f32::from(self.0) / 1000.0
    }
}

/// Modo de dimensionamiento en un eje (modelo flex simple).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Length {
    /// Se ajusta al contenido intrínseco del widget (por defecto).
    Auto,
    /// Tamaño fijo en píxeles lógicos a 96 DPI.
    Fixed(u16),
    /// Reparte el espacio sobrante del contenedor padre en proporción al peso (`>= 1`).
    Fill(u8),
}

/// Márgenes internos en píxeles lógicos a 96 DPI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Edges {
    pub top: u16,
    pub right: u16,
    pub bottom: u16,
    pub left: u16,
}

/// Alineación transversal de los hijos dentro de un `Row` o `Column`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrossAlign {
    Start,
    Center,
    End,
    Stretch,
}

/// Alineación en el eje principal cuando sobra espacio y no hay hijos `Fill`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainAlign {
    Start,
    Center,
    End,
    SpaceBetween,
}

/// Rol semántico de color para texto o íconos, resuelto contra el tema activo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// Color principal de texto (`text_color` del tema).
    Primary,
    /// Color secundario / atenuado (por ejemplo para la fecha o subtítulos).
    Muted,
    /// Color de acento para controles activos.
    Accent,
    /// Color explícito RGBA.
    Custom(atoll_render::Color),
}

/// Variante visual de un botón.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonVariant {
    /// Fondo sutil elevado sobre la superficie.
    Secondary,
    /// Fondo con color de acento.
    Primary,
    /// Transparente en reposo; muestra fondo suave solo en hover/presionado (ideal para íconos en la isla).
    Ghost,
}

/// Nodo del árbol de widgets (100 % datos planos, sin closures ni referencias).
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
    /// Especie del widget y sus datos propios.
    pub kind: WidgetKind,
}

/// Los 10 widgets base de la v1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WidgetKind {
    /// Texto de una sola línea.
    Label {
        text: String,
        size_px: u16,
        tone: Tone,
    },
    /// Ícono vectorial predefinido, sin fuentes externas.
    Icon {
        icon: IconId,
        size_px: u16,
        tone: Tone,
    },
    /// Contenedor horizontal.
    Row {
        gap: u16,
        main_align: MainAlign,
        cross_align: CrossAlign,
        children: Vec<Node>,
    },
    /// Contenedor vertical.
    Column {
        gap: u16,
        main_align: MainAlign,
        cross_align: CrossAlign,
        children: Vec<Node>,
    },
    /// Espacio vacío elástico o fijo dentro de un `Row` o `Column`.
    Spacer,
    /// Botón clicleable con texto opcional e ícono opcional.
    Button {
        label: Option<String>,
        icon: Option<IconId>,
        variant: ButtonVariant,
        enabled: bool,
    },
    /// Interruptor booleano (encendido / apagado).
    Toggle {
        checked: bool,
        enabled: bool,
    },
    /// Control deslizable continuo con valor normalizado `0..=1000`.
    Slider {
        value: Permille,
        enabled: bool,
    },
    /// Barra de progreso de solo lectura (`0..=1000`).
    Progress {
        value: Permille,
        tone: Tone,
    },
    /// Línea divisoria horizontal (en `Column`) o vertical (en `Row`).
    Separator {
        thickness_px: u16,
    },
}
```

---

## 4. Solución para íconos vectoriales y presupuesto de memoria

### Íconos sin fuentes (`IconId`)
En lugar de embeber una fuente de íconos (que sumaría decenas de KB al binario y entradas en la caché de glifos de `fontdue`), `atoll-ui` define un `enum IconId` compacto cuyos trazos se dibujan directamente con `tiny-skia` (`PathBuilder` con líneas, arcos y rectángulos redondeados en una grilla normalizada de `24x24` escalada a `size_px * scale`):

```rust
/// Conjunto cerrado de íconos vectoriales predefinidos (v1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IconId {
    Play,
    Pause,
    NextTrack,
    PreviousTrack,
    Volume,
    VolumeMute,
    Settings,
    Check,
    Close,
    ChevronRight,
    ChevronDown,
}
```

- **Costo de memoria de los íconos:** **0 bytes en heap y 0 bytes de caché**. Son funciones estáticas que emiten entre 4 y 12 segmentos en `tiny-skia::PathBuilder` en el momento de pintar.
- **Costo en el binario:** ~1,5 KB de código máquina para las 11 formas vectoriales.

### Presupuesto de memoria del árbol y del layout
Calculemos el tamaño en memoria en `x86_64` para un árbol grande (por ejemplo, **250 widgets** en `atoll-settings` o varios plugins activos):

- `Node`:
  - `id` (`4 B`) + `width` (`4 B`) + `height` (`4 B`) + `padding` (`8 B`) + `WidgetKind` (`~40 B` dominado por `Vec<Node>` o `String` de 24 B) = **`64 bytes` por `Node`**.
  - 250 nodos × 64 bytes = **16 KB** para la estructura del árbol, más ~4 KB de strings cortos (`Label` / `Button`) = **~20 KB**.
- `LayoutTree` (resultado plano del layout):
  - Cada `LayoutBox` guarda `{ id: WidgetId, rect: Rect, clip: Rect }` = **`36 bytes` por nodo**.
  - 250 nodos × 36 bytes = **9 KB** en un único `Vec<LayoutBox>` contiguo.
- `UiState` (estado interactivo):
  - `hovered: Option<WidgetId>`, `pressed: Option<WidgetId>`, `dragging: Option<WidgetId>` = **`16 bytes` en total** (no crece con la cantidad de widgets).

**Total para 250 widgets:** **~29 KB de RAM** (menos del 0,2 % del presupuesto total de 20 MB de Atoll, muy por debajo del tope de pocos cientos de KB).

---

## 5. Modelo de layout puro (Capa 2)

El layout se resuelve en dos pasadas simples sin recursión profunda descontrolada (tope de profundidad `MAX_TREE_DEPTH = 32` para evitar desbordes si un plugin malicioso envía un árbol degenerado) y se apoya en un trait `TextMeasurer` para que pueda testearse tanto con `atoll-render::TextRenderer` real como con un medidor fijo determinista en tests unitarios rápidos:

```rust
/// Rectángulo en píxeles físicos enteros dentro del `Frame`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    /// Comprueba si el punto físico `(px, py)` cae dentro del rectángulo.
    #[must_use]
    pub const fn contains(self, px: i32, py: i32) -> bool {
        px >= self.x
            && py >= self.y
            && (px - self.x) < (self.width as i32)
            && (py - self.y) < (self.height as i32)
    }
}

/// Abstracción mínima para medir texto durante el layout.
pub trait TextMeasurer {
    /// Devuelve el ancho y alto en píxeles físicos para `text` al tamaño físico `px`.
    fn measure_line(&mut self, text: &str, px: f32) -> Size;
}

/// Nodo ya posicionado en coordenadas físicas, listo para hit-testing y pintado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutNode {
    pub id: WidgetId,
    pub rect: Rect,
    pub content_rect: Rect,
    pub kind: LayoutKind,
}

/// Árbol posicionado resultante de aplicar layout a un `Node`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutTree {
    pub root_size: Size,
    pub scale: u32, // bits o milésimas para comparar igualdad
    pub nodes: Vec<LayoutNode>,
}

/// Calcula las posiciones físicas de todos los nodos dentro de `available` a la escala `scale`.
#[must_use]
pub fn compute_layout(
    root: &Node,
    available: Size,
    scale: f32,
    measurer: &mut dyn TextMeasurer,
) -> LayoutTree;
```

### Reglas del algoritmo de layout
1. **Tamaño intrínseco (`Length::Auto`):**
   - `Label`: ancho y alto devueltos por `TextMeasurer` más `padding`.
   - `Icon`: `size_px * scale` más `padding`.
   - `Button`: suma de ícono + separación + texto + relleno estándar (`12x6` px lógicos), con altura mínima de `28` px lógicos.
   - `Toggle`: `36x20` px lógicos.
   - `Slider`: ancho mínimo `80` px lógicos (o `Fill` si se indica), alto `20` px lógicos.
   - `Progress`: ancho mínimo `60` px lógicos, alto `6` px lógicos.
   - `Separator`: `thickness_px` en el eje principal del padre y `CrossAlign::Stretch` en el eje transversal.
2. **Reparto en `Row` y `Column`:**
   - Se descuentan `padding` del contenedor y `(n - 1) * gap`.
   - Primero se miden todos los hijos con `Length::Fixed` y `Length::Auto`.
   - El espacio restante en el eje principal se divide proporcionalmente entre los hijos con `Length::Fill(weight)`.
   - Los hijos se ubican según `MainAlign` y `CrossAlign`.

---

## 6. Estado interactivo, hit-testing y eventos (Capa 3)

El árbol `Node` es inmutable entre actualizaciones de estado; el estado transitorio del puntero (qué widget tiene hover, cuál está presionado y cuál se está arrastrando) vive en una estructura liviana `UiState` propiedad de la ventana:

```rust
/// Entrada del puntero entregada por el backend de la plataforma (en píxeles físicos).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerInput {
    /// El puntero se movió a `(x, y)` dentro de la ventana.
    Move { x: i32, y: i32 },
    /// El puntero salió de la ventana (`WM_MOUSELEAVE`).
    Leave,
    /// Se presionó el botón izquierdo en `(x, y)`.
    Down { x: i32, y: i32 },
    /// Se soltó el botón izquierdo en `(x, y)`.
    Up { x: i32, y: i32 },
}

/// Evento semántico emitido por un widget cuando el usuario interactúa con él.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiEvent {
    /// Se hizo clic completo (presionar y soltar sobre el mismo `Button`).
    Clicked(WidgetId),
    /// Un `Toggle` cambió su valor booleano al hacerle clic.
    Toggled { id: WidgetId, checked: bool },
    /// Un `Slider` cambió su valor (al hacer clic o arrastrar).
    SliderChanged { id: WidgetId, value: Permille },
}

/// Resultado de procesar una entrada del puntero.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InteractionOutcome {
    /// `true` si cambió algún estado visual (`hovered`, `pressed` o valor de slider)
    /// y por lo tanto hay que redibujar una vez.
    pub needs_redraw: bool,
    /// Eventos de alto nivel generados en este paso (0 o 1 en la práctica).
    pub events: Vec<UiEvent>,
}

/// Estado interactivo efímero de la interfaz.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UiState {
    hovered: Option<WidgetId>,
    pressed: Option<WidgetId>,
    dragging_slider: Option<WidgetId>,
}

impl UiState {
    /// Procesa un evento del puntero contra el `LayoutTree` actual sin tocar el SO.
    pub fn handle_pointer(
        &mut self,
        layout: &LayoutTree,
        input: PointerInput,
    ) -> InteractionOutcome;

    /// Indica si `id` tiene el puntero encima.
    #[must_use]
    pub fn is_hovered(&self, id: WidgetId) -> bool;

    /// Indica si `id` está siendo presionado o arrastrado.
    #[must_use]
    pub fn is_active(&self, id: WidgetId) -> bool;
}
```

### Semántica de interacción
- **Hover:** `PointerInput::Move` busca el último widget interactivo habilitado (`enabled == true` y `id != WidgetId::NONE`) cuyo `rect` contenga `(x, y)`. Solo si el `hovered` resultante es distinto del anterior, `needs_redraw = true` (evita redibujar en cada píxel que se mueve el mouse dentro del mismo botón o sobre zonas sin widgets interactivos).
- **Clic (`Button` y `Toggle`):** `Down` sobre un widget habilitado guarda `pressed = Some(id)` (`needs_redraw = true`). `Up` emite `UiEvent::Clicked(id)` o `UiEvent::Toggled { id, checked: ! current }` únicamente si el puntero sigue dentro del `rect` de ese mismo `id`; en todos los casos limpia `pressed = None`.
- **Arrastre (`Slider`):** `Down` sobre un `Slider` habilitado fija `dragging_slider = Some(id)`, calcula el `Permille` según `(x - rect.x) / rect.width` acotado a `0..=1000`, y emite `UiEvent::SliderChanged`. Mientras `dragging_slider == Some(id)`, cualquier `Move` actualiza el `Permille` (incluso si el puntero se desvía verticalmente fuera del slider) y solo marca `needs_redraw = true` si el `Permille` cambió. `Up` o `Leave` termina el arrastre.

---

## 7. Aplicación de colores del tema y pintado (Capa 4)

`ThemePalette` deriva todos los colores de los widgets a partir de `atoll_config::AppearanceSection` (`background`, `text_color`, `opacity`), garantizando contraste tanto en temas oscuros (`black`, `graphite`) como claros (`white`) y en colores personalizados:

```rust
/// Paleta de colores resuelta a partir de `AppearanceSection` de `atoll-config`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThemePalette {
    /// Fondo principal de la superficie (con opacidad aplicada).
    pub background: Color,
    /// Color principal del texto (`Tone::Primary`).
    pub text_primary: Color,
    /// Color secundario del texto (`Tone::Muted`, ~75 % del alfa de `text_primary`).
    pub text_muted: Color,
    /// Superficie de controles en reposo (mezcla sutil de `text_primary` al 12 % sobre el fondo).
    pub control_bg: Color,
    /// Superficie de controles en hover (mezcla al 20 %).
    pub control_hover_bg: Color,
    /// Superficie de controles presionados (mezcla al 28 %).
    pub control_active_bg: Color,
    /// Color de acento para `ButtonVariant::Primary`, `Toggle` activo, `Slider` y `Progress`.
    pub accent: Color,
    /// Color de líneas divisorias (`Separator`, mezcla al 15 %).
    pub separator: Color,
}

impl ThemePalette {
    /// Construye la paleta a partir de la sección `[appearance]` de `atoll-config`.
    #[must_use]
    pub fn from_appearance(appearance: &atoll_config::AppearanceSection) -> Self;
}

/// Dibuja un `LayoutTree` sobre un `Frame` ya inicializado.
pub fn paint_tree(
    frame: &mut atoll_render::Frame,
    layout: &LayoutTree,
    state: &UiState,
    palette: &ThemePalette,
    text: &mut atoll_render::TextRenderer,
) -> Result<(), atoll_render::RenderError>;
```

---

## 8. Plan de Pull Requests, criterios de aceptación y tests

Para mantener PRs chicos, revisables y atómicos, el ítem 14 se divide en **4 PRs secuenciales**:

### PR 14a — `feat/atoll-ui-layout`: crate `packages/atoll-ui`, árbol de datos y motor de layout
- **Alcance:**
  - Crear `packages/atoll-ui` y registrarlo en el workspace (`Cargo.toml`, `docs/ARCHITECTURE.md`, `AGENTS.md`).
  - Implementar `WidgetId`, `Permille`, `Length`, `Edges`, `MainAlign`, `CrossAlign`, `Node`, `WidgetKind` y constructores ergonómicos (`Node::label`, `Node::row`, `Node::column`, `Node::button`, etc.).
  - Implementar `compute_layout` con soporte completo de `Row`, `Column`, `Spacer`, `Padding`, `Length::{Auto, Fixed, Fill}`, `MainAlign`, `CrossAlign` y límite de profundidad `MAX_TREE_DEPTH`.
- **Tests (corren en Linux sin ventana):**
  - `Row` y `Column` con hijos `Fixed`, `Auto` y múltiples `Fill` con distintos pesos.
  - Alineaciones (`Start`, `Center`, `End`, `SpaceBetween`, `Stretch`).
  - Escalado DPI (`1.0`, `1.25`, `1.5`, `2.0`).
  - Espacio insuficiente (saturación sin underflow ni pánicos) y árbol que supera `MAX_TREE_DEPTH`.
  - Test de tamaño en memoria (`size_of::<Node>() <= 80`).
- **Aceptación:** `fmt`, `clippy -D warnings` y `test` en verde en los 3 SO; 0 dependencias nuevas.

### PR 14b — `feat/atoll-ui-eventos`: hit-testing y máquina de estados interactiva
- **Alcance:**
  - Implementar `PointerInput`, `UiEvent`, `InteractionOutcome` y `UiState::handle_pointer`.
- **Tests (corren en Linux sin ventana):**
  - Hover al entrar, moverse dentro del mismo widget (`needs_redraw == false`) y pasar a otro widget (`needs_redraw == true`).
  - Clic en `Button` (presionar y soltar adentro emite `Clicked`; presionar adentro y soltar afuera cancela sin emitir evento).
  - Clic en `Toggle` invierte el booleano en `UiEvent::Toggled`.
  - Arrastre de `Slider` actualiza `Permille` en los extremos (`0` y `1000`) y en puntos intermedios, y no pide redibujar si el valor no cambió.
  - Widgets con `enabled: false` ignoran hover, clic y arrastre.
- **Aceptación:** 100 % determinista y portable en Linux.

### PR 14c — `feat/atoll-ui-render`: `ThemePalette`, íconos vectoriales y pintado sobre `Frame`
- **Alcance:**
  - Implementar `ThemePalette::from_appearance`.
  - Implementar el trazado de los 11 `IconId` y de todos los widgets (`Label`, `Icon`, `Button`, `Toggle`, `Slider`, `Progress`, `Separator`) sobre `atoll-render::Frame`.
- **Tests (píxeles en memoria en Linux):**
  - Cada uno de los 10 widgets y los 11 íconos modifica píxeles dentro de su `Rect` y nunca escribe fuera de él.
  - Un `Button` en estado normal, hover y presionado produce colores distintos.
  - Un `Toggle` en `false` y `true` dibuja la perilla a la izquierda y a la derecha respectivamente.
  - Un `Slider` y un `Progress` a `0`, `500` y `1000` llenan la proporción correcta de su barra.
  - Todos los píxeles del `Frame` mantienen alfa premultiplicado válido (`r, g, b <= a`).
- **Aceptación:** tests de píxeles en verde en Linux, Windows y macOS.

### PR 14d — `feat/atoll-ui-isla`: integración en `IslandPainter` con contenido real
- **Alcance:**
  - Reemplazar el posicionamiento manual de hora y fecha en `atoll-core::island::IslandPainter` por un árbol de `atoll-ui` (`Column` centrada con `Label` de hora y `Label` de fecha durante la expansión), conectando los eventos de mouse de la ventana cuando corresponda.
- **Tests y verificación:**
  - Todos los tests existentes de `IslandPainter` siguen pasando.
  - Prueba manual en Windows y medición de RAM (`< 20 MB`) y CPU en reposo (`0,00` entre dos muestras a 30 s).

---

## 9. Riesgos y mitigaciones

1. **Redibujados innecesarios al mover el mouse (`WM_MOUSEMOVE`):**
   - *Riesgo:* Windows envía decenas de `WM_MOUSEMOVE` por segundo mientras el cursor se mueve sobre la isla. Si cada uno redibujara el `Frame`, subiría el uso de CPU.
   - *Mitigación:* `UiState::handle_pointer` devuelve `needs_redraw: false` si el `WidgetId` bajo el cursor no cambió (y no hay un `Slider` arrastrándose cuyo valor haya cambiado). En la isla actual (solo reloj y fecha, sin widgets interactivos aún), mover el mouse adentro produce **cero** redibujados extra tras terminar la animación.
2. **Árboles patológicos enviados por un plugin (ítem 15):**
   - *Riesgo:* un plugin con un bug podría armar un árbol de 10.000 nodos o 500 niveles de anidamiento.
   - *Mitigación:* `compute_layout` aplica dos límites duros constantes: `MAX_TREE_DEPTH = 32` y `MAX_TREE_NODES = 1024`, truncando el resto sin entrar en pánico ni desbordar la pila.
3. **Interpolación suave durante la animación de hover de la isla:**
   - *Riesgo:* mientras la isla se expande de `400x40` a `480x64`, la fecha aparece con un fundido de opacidad (`alpha` proporcional al progreso).
   - *Mitigación:* `Tone::Custom(Color)` permite que `IslandPainter` pase el color de la fecha con su alfa interpolado en cada paso de los 180 ms de animación, manteniendo la misma transición visual que hoy.

---

## 10. Preguntas abiertas para decidir antes de programar el PR 14a

Mi recomendación está indicada en cada punto; confirmame si estás de acuerdo o si preferís ajustarla:

1. **Color de acento (`accent`) en `ThemePalette`:**
   - Hoy `[appearance]` en `config.ini` tiene `background` y `text_color`, pero no `accent_color`.
   - **Recomendación:** en el ítem 14 derivar `accent` por preset (`#0A84FF` azul sistema en `black` y `graphite`, `#0066CC` en `white`) sin tocar `config.ini`, o bien agregar `accent_color` opcional en `[appearance]`. Recomiendo **derivarlo del preset por ahora** para no tocar `atoll-config` en el 14a y evaluar si vale la pena exponerlo en `config.ini` cuando hagamos `atoll-settings` (ítem 17).
2. **Lista inicial de `IconId` (11 íconos):**
   - Propuse `Play`, `Pause`, `NextTrack`, `PreviousTrack`, `Volume`, `VolumeMute`, `Settings`, `Check`, `Close`, `ChevronRight`, `ChevronDown` pensando en el plugin de música (ítem 18) y `atoll-settings` (ítem 17).
   - **Recomendación:** arrancar con esos 11 en el PR 14c y sumar nuevos variantes al enum a medida que un plugin concreto los necesite.
3. **Clic izquierdo vs. clic derecho en la ventana:**
   - Hoy el clic derecho (`WM_RBUTTONUP`) cierra la isla.
   - **Recomendación:** mantener el clic derecho reservado para cerrar la isla (y en el futuro abrir el menú contextual / configuración), y que `atoll-ui` procese únicamente el **clic izquierdo** (`WM_LBUTTONDOWN` / `WM_LBUTTONUP`) para botones, toggles y sliders.
