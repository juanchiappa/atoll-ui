//! Protocolo de IPC local entre el Core de Atoll y los plugins.
//!
//! Los plugins son procesos separados: toda la comunicación pasa por acá.

/// Versión actual del protocolo. Se incrementa con cada cambio incompatible.
pub const PROTOCOL_VERSION: u32 = 1;

#[cfg(test)]
mod tests {
    use super::PROTOCOL_VERSION;

    #[test]
    fn protocol_version_starts_at_one() {
        assert_eq!(PROTOCOL_VERSION, 1);
    }
}
