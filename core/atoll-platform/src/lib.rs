//! Capa de plataforma de Atoll.
//!
//! Todo el código específico de cada sistema operativo vive acá,
//! detrás de `#[cfg(target_os = "...")]`.

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

#[cfg(test)]
mod tests {
    use super::Os;

    #[test]
    fn current_os_is_supported() {
        assert_ne!(Os::current(), Os::Unknown);
    }
}
