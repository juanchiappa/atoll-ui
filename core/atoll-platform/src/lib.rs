//! Capa de plataforma de Atoll.
//!
//! Todo el código específico de cada sistema operativo vive acá,
//! detrás de `#[cfg(target_os = "...")]`. El resto del proyecto solo
//! conoce el trait [`Backend`].

use std::fmt;

use atoll_render::Frame;

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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IslandConfig {
    /// Ancho en píxeles.
    pub width: u32,
    /// Alto en píxeles.
    pub height: u32,
    /// Separación en píxeles entre el borde superior de la pantalla y la isla.
    pub top_margin: u32,
}

impl IslandConfig {
    /// Crea una configuración a partir de tamaño y margen superior.
    #[must_use]
    pub const fn new(width: u32, height: u32, top_margin: u32) -> Self {
        Self {
            width,
            height,
            top_margin,
        }
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

/// Backend nativo de un sistema operativo.
///
/// Un backend abre la ventana de la isla y bloquea el hilo hasta que se
/// cierra. Mientras no haya eventos, el hilo duerme: no hay render loop.
pub trait Backend {
    /// Muestra `frame` como isla y procesa eventos hasta que el usuario la cierra.
    ///
    /// # Errors
    ///
    /// Devuelve [`PlatformError`] si el sistema operativo rechaza alguna llamada
    /// o si `frame` no tiene el tamaño de `config`.
    fn run(&mut self, config: &IslandConfig, frame: &Frame) -> Result<(), PlatformError>;
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
    use super::{IslandConfig, Os, PlatformError};

    #[test]
    fn current_os_is_supported() {
        assert_ne!(Os::current(), Os::Unknown);
    }

    #[test]
    fn config_keeps_values() {
        let config = IslandConfig::new(400, 40, 8);
        assert_eq!(
            (config.width, config.height, config.top_margin),
            (400, 40, 8)
        );
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
