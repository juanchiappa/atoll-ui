//! Capa de plataforma de Atoll.
//!
//! Todo el código específico de cada sistema operativo vive acá,
//! detrás de `#[cfg(target_os = "...")]`. El resto del proyecto solo
//! conoce el trait [`Backend`].

use std::fmt;
use std::path::PathBuf;

use atoll_render::{Frame, Size};

pub use clock::{local_time, LocalTime};

pub mod animation;
pub mod clock;
pub mod dpi;

#[cfg(windows)]
mod windows;

/// Sistema operativo sobre el que corre Atoll.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    /// Microsoft Windows.
    Windows,
    /// Linux (X11 o Wayland).
    Linux,
    /// Apple macOS.
    MacOs,
    /// Sistema no soportado oficialmente.
    Unknown,
}

impl Os {
    /// Devuelve el sistema operativo para el que se compiló Atoll.
    #[must_use]
    pub const fn current() -> Self {
        if cfg!(target_os = "windows") {
            Self::Windows
        } else if cfg!(target_os = "linux") {
            Self::Linux
        } else if cfg!(target_os = "macos") {
            Self::MacOs
        } else {
            Self::Unknown
        }
    }
}

/// Configuración de la ventana de la isla.
///
/// Todas las medidas son **lógicas**: píxeles a 96 DPI (100 % de escala). El
/// backend las multiplica por el factor de escala del monitor (ver [`dpi`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IslandConfig {
    /// Tamaño lógico de la isla en reposo.
    pub size: Size,
    /// Tamaño lógico de la isla cuando el mouse está encima.
    pub hover_size: Size,
    /// Separación lógica entre el borde superior de la pantalla y la isla.
    pub top_margin: u32,
    /// Carpeta donde vive `config.ini`, para observar cambios y recargar en vivo.
    pub config_dir: Option<PathBuf>,
}

impl IslandConfig {
    /// Crea una configuración a partir de los dos tamaños y el margen superior,
    /// sin carpeta de recarga en vivo.
    #[must_use]
    pub const fn new(size: Size, hover_size: Size, top_margin: u32) -> Self {
        Self {
            size,
            hover_size,
            top_margin,
            config_dir: None,
        }
    }

    /// Asocia la carpeta de configuración para habilitar la recarga en vivo.
    #[must_use]
    pub fn with_config_dir(mut self, dir: PathBuf) -> Self {
        self.config_dir = Some(dir);
        self
    }
}

/// Errores de la capa de plataforma.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlatformError {
    /// Todavía no hay backend para este sistema operativo.
    Unsupported(Os),
    /// La imagen a mostrar no coincide con la configuración de la isla.
    InvalidFrame,
    /// Falló una llamada al sistema operativo.
    Os {
        /// Nombre de la función del SO que falló.
        call: &'static str,
        /// Código de error devuelto por el SO.
        code: u32,
    },
}

impl fmt::Display for PlatformError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported(os) => write!(f, "todavía no hay backend para {os:?}"),
            Self::InvalidFrame => write!(f, "la imagen no coincide con el tamaño de la isla"),
            Self::Os { call, code } => write!(f, "falló {call} (código de error {code})"),
        }
    }
}

impl std::error::Error for PlatformError {}

/// Dibuja el contenido de la isla y gestiona su recarga de configuración.
///
/// El backend lo llama cada vez que hay que mostrar algo nuevo: al abrir, en cada
/// cuadro de la animación de hover, cuando cambia el minuto, cuando cambia el
/// DPI del monitor y cuando se recarga el archivo de configuración. Quién implementa
/// este trait decide qué se ve; el backend solo lo entrega al sistema operativo.
pub trait Painter {
    /// Dibuja la isla con la hora `time`.
    ///
    /// `size` es el tamaño **físico** del cuadro, en píxeles. `scale` es el factor
    /// de escala del monitor (1,0 = 100 %, 1,5 = 150 %): sirve para escalar texto,
    /// radio de borde y separaciones, que se definen en píxeles lógicos.
    ///
    /// Devuelve `None` si no se pudo dibujar.
    fn paint(&mut self, size: Size, scale: f32, time: LocalTime) -> Option<Frame>;

    /// Se invoca cuando el archivo de configuración cambió en disco (tras el debounce).
    ///
    /// Si el archivo se pudo leer y la configuración resultante **cambió** respecto
    /// de la actual, aplica los nuevos valores y devuelve `Some(IslandConfig)` con
    /// las nuevas medidas lógicas para que el backend reconfigure la ventana y
    /// redibuje una sola vez. Si no se pudo leer o nada cambió, devuelve `None`.
    fn reload_config(&mut self) -> Option<IslandConfig> {
        None
    }
}

/// Backend nativo de un sistema operativo.
///
/// Un backend abre la ventana de la isla y bloquea el hilo hasta que se
/// cierra. Mientras no haya eventos, el hilo duerme: no hay render loop.
pub trait Backend {
    /// Muestra la isla dibujada por `painter` y procesa eventos hasta que el usuario la cierra.
    ///
    /// Al pasar el mouse la isla se expande a `config.hover_size` con una
    /// animación, y vuelve a su tamaño al sacarlo. El backend mantiene un único
    /// timer de un solo disparo, alineado al cambio de minuto, para redibujar la hora.
    /// Si cambia el DPI del monitor o el archivo de configuración, recalcula los
    /// tamaños físicos y vuelve a dibujar una sola vez.
    ///
    /// # Errors
    ///
    /// Devuelve [`PlatformError`] si el sistema operativo rechaza alguna llamada
    /// o si el primer cuadro no tiene el tamaño de `config.size` escalado.
    fn run(
        &mut self,
        config: &IslandConfig,
        painter: Box<dyn Painter>,
    ) -> Result<(), PlatformError>;
}

/// Devuelve el backend nativo del sistema operativo actual.
///
/// # Errors
///
/// Devuelve [`PlatformError::Unsupported`] si todavía no existe un backend
/// para este sistema operativo.
#[cfg(windows)]
pub fn native_backend() -> Result<Box<dyn Backend>, PlatformError> {
    Ok(Box::new(windows::WindowsBackend))
}

/// Devuelve el backend nativo del sistema operativo actual.
///
/// # Errors
///
/// Devuelve [`PlatformError::Unsupported`] si todavía no existe un backend
/// para este sistema operativo.
#[cfg(not(windows))]
pub fn native_backend() -> Result<Box<dyn Backend>, PlatformError> {
    Err(PlatformError::Unsupported(Os::current()))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{IslandConfig, Os, PlatformError};
    use atoll_render::Size;

    #[test]
    fn current_os_is_supported() {
        assert_ne!(Os::current(), Os::Unknown);
    }

    #[test]
    fn config_keeps_values() {
        let config = IslandConfig::new(Size::new(400, 40), Size::new(480, 64), 8)
            .with_config_dir(PathBuf::from("Atoll"));
        assert_eq!(config.size, Size::new(400, 40));
        assert_eq!(config.hover_size, Size::new(480, 64));
        assert_eq!(config.top_margin, 8);
        assert_eq!(config.config_dir, Some(PathBuf::from("Atoll")));
    }

    #[test]
    fn error_messages_are_readable() {
        let err = PlatformError::Os {
            call: "CreateWindowExW",
            code: 5,
        };
        assert_eq!(err.to_string(), "falló CreateWindowExW (código de error 5)");
    }

    #[cfg(not(windows))]
    #[test]
    fn native_backend_is_unsupported_outside_windows() {
        assert!(matches!(
            super::native_backend(),
            Err(PlatformError::Unsupported(_))
        ));
    }
}
