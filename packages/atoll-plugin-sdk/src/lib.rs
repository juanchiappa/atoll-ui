//! SDK para escribir plugins de Atoll.

/// Versión del protocolo de IPC que habla este SDK.
#[must_use]
pub const fn protocol_version() -> u32 {
    atoll_ipc::PROTOCOL_VERSION
}

#[cfg(test)]
mod tests {
    use super::protocol_version;

    #[test]
    fn sdk_matches_ipc_protocol() {
        assert_eq!(protocol_version(), atoll_ipc::PROTOCOL_VERSION);
    }
}
