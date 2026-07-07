//! MySQL-compatible protocol adapter for SQL Lens.

mod handshake;
mod packet;

use sql_lens_core::ProtocolName;
use sql_lens_protocol::{
    CaptureEventEmitter, ProtocolAdapter, ProtocolAdapterError, ProtocolConnectionContext,
    ProtocolConnectionState, ProtocolObservation,
};

pub use handshake::{MysqlHandshakeParseError, MysqlInitialHandshake, parse_initial_handshake};
pub use packet::{
    MYSQL_PACKET_HEADER_LEN, MysqlPacket, MysqlPacketHeader, MysqlPacketParseError,
    parse_mysql_packet,
};

pub const MYSQL_PROTOCOL_NAME: &str = "mysql";

#[derive(Debug, Clone, Copy, Default)]
pub struct MysqlProtocolAdapter;

impl MysqlProtocolAdapter {
    pub fn new() -> Self {
        Self
    }

    fn state_mut<'a>(
        &self,
        state: &'a mut dyn ProtocolConnectionState,
    ) -> Result<&'a mut MysqlConnectionState, ProtocolAdapterError> {
        state
            .as_any_mut()
            .downcast_mut::<MysqlConnectionState>()
            .ok_or(ProtocolAdapterError::InvalidConnectionState {
                expected: "MysqlConnectionState",
            })
    }
}

impl ProtocolAdapter for MysqlProtocolAdapter {
    fn protocol_name(&self) -> ProtocolName {
        ProtocolName(MYSQL_PROTOCOL_NAME.to_owned())
    }

    fn create_connection_state(
        &self,
        _context: &ProtocolConnectionContext,
    ) -> Box<dyn ProtocolConnectionState> {
        Box::new(MysqlConnectionState::default())
    }

    fn observe_client_bytes(
        &self,
        state: &mut dyn ProtocolConnectionState,
        bytes: &[u8],
        _events: &mut dyn CaptureEventEmitter,
    ) -> Result<ProtocolObservation, ProtocolAdapterError> {
        let state = self.state_mut(state)?;
        state.client_bytes_observed += bytes.len();

        Ok(ProtocolObservation::new(bytes.len(), 0))
    }

    fn observe_backend_bytes(
        &self,
        state: &mut dyn ProtocolConnectionState,
        bytes: &[u8],
        _events: &mut dyn CaptureEventEmitter,
    ) -> Result<ProtocolObservation, ProtocolAdapterError> {
        let state = self.state_mut(state)?;
        state.backend_bytes_observed += bytes.len();
        state.observe_initial_handshake(bytes);

        Ok(ProtocolObservation::new(bytes.len(), 0))
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MysqlConnectionPhase {
    #[default]
    AwaitingInitialHandshake,
    InitialHandshakeSeen,
}

#[derive(Debug, Default)]
pub struct MysqlConnectionState {
    client_bytes_observed: usize,
    backend_bytes_observed: usize,
    phase: MysqlConnectionPhase,
    initial_handshake: Option<MysqlInitialHandshake>,
}

impl MysqlConnectionState {
    pub fn client_bytes_observed(&self) -> usize {
        self.client_bytes_observed
    }

    pub fn backend_bytes_observed(&self) -> usize {
        self.backend_bytes_observed
    }

    pub fn phase(&self) -> MysqlConnectionPhase {
        self.phase
    }

    pub fn initial_handshake(&self) -> Option<&MysqlInitialHandshake> {
        self.initial_handshake.as_ref()
    }

    fn observe_initial_handshake(&mut self, bytes: &[u8]) {
        if self.phase != MysqlConnectionPhase::AwaitingInitialHandshake {
            return;
        }

        let Ok(packet) = parse_mysql_packet(bytes) else {
            return;
        };

        if packet.header.sequence_id != 0 {
            return;
        }

        let Ok(handshake) = parse_initial_handshake(packet.payload) else {
            return;
        };

        self.initial_handshake = Some(handshake);
        self.phase = MysqlConnectionPhase::InitialHandshakeSeen;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sql_lens_core::{
        ConnectionId, ConnectionInfo, ConnectionState, DatabaseType, ProtocolName, SqlEvent,
        Timestamp,
    };
    use sql_lens_protocol::{CaptureEventEmitter, ProtocolAdapterRegistry};

    #[derive(Debug, Default)]
    struct VecCaptureEventEmitter {
        events: Vec<SqlEvent>,
    }

    impl CaptureEventEmitter for VecCaptureEventEmitter {
        fn emit(&mut self, event: SqlEvent) {
            self.events.push(event);
        }
    }

    #[test]
    fn mysql_adapter_reports_mysql_protocol_name() {
        let adapter = MysqlProtocolAdapter::new();

        assert_eq!(
            adapter.protocol_name(),
            ProtocolName(MYSQL_PROTOCOL_NAME.to_owned())
        );
    }

    #[test]
    fn mysql_adapter_registers_as_mysql() {
        let mut registry = ProtocolAdapterRegistry::new();

        registry
            .register(MysqlProtocolAdapter::new())
            .expect("mysql adapter should register");

        assert!(registry.contains(&ProtocolName(MYSQL_PROTOCOL_NAME.to_owned())));
        assert!(
            registry
                .resolve(&ProtocolName(MYSQL_PROTOCOL_NAME.to_owned()))
                .is_ok()
        );
    }

    #[test]
    fn mysql_adapter_creates_mysql_connection_state() {
        let adapter = MysqlProtocolAdapter::new();
        let state = adapter.create_connection_state(&test_context());

        assert!(state.as_ref().as_any().is::<MysqlConnectionState>());
    }

    #[test]
    fn mysql_connection_state_starts_awaiting_initial_handshake() {
        let adapter = MysqlProtocolAdapter::new();
        let state = adapter.create_connection_state(&test_context());
        let state = state
            .as_ref()
            .as_any()
            .downcast_ref::<MysqlConnectionState>()
            .expect("state should downcast");

        assert_eq!(
            state.phase(),
            MysqlConnectionPhase::AwaitingInitialHandshake
        );
        assert!(state.initial_handshake().is_none());
    }

    #[test]
    fn mysql_adapter_observes_client_and_backend_bytes_without_events() {
        let adapter = MysqlProtocolAdapter::new();
        let mut state = adapter.create_connection_state(&test_context());
        let mut events = VecCaptureEventEmitter::default();

        let client_observation = adapter
            .observe_client_bytes(state.as_mut(), b"client", &mut events)
            .expect("client bytes should be observed");
        let backend_observation = adapter
            .observe_backend_bytes(state.as_mut(), b"backend", &mut events)
            .expect("backend bytes should be observed");
        let state = state
            .as_ref()
            .as_any()
            .downcast_ref::<MysqlConnectionState>()
            .expect("state should downcast");

        assert_eq!(client_observation, ProtocolObservation::new(6, 0));
        assert_eq!(backend_observation, ProtocolObservation::new(7, 0));
        assert_eq!(state.client_bytes_observed(), 6);
        assert_eq!(state.backend_bytes_observed(), 7);
        assert_eq!(
            state.phase(),
            MysqlConnectionPhase::AwaitingInitialHandshake
        );
        assert!(events.events.is_empty());
    }

    #[test]
    fn mysql_adapter_observes_backend_initial_handshake() {
        let adapter = MysqlProtocolAdapter::new();
        let mut state = adapter.create_connection_state(&test_context());
        let mut events = VecCaptureEventEmitter::default();
        let packet = initial_handshake_packet();

        let observation = adapter
            .observe_backend_bytes(state.as_mut(), &packet, &mut events)
            .expect("backend handshake should be observed");
        let state = state
            .as_ref()
            .as_any()
            .downcast_ref::<MysqlConnectionState>()
            .expect("state should downcast");
        let handshake = state
            .initial_handshake()
            .expect("initial handshake should be stored");

        assert_eq!(observation, ProtocolObservation::new(packet.len(), 0));
        assert_eq!(state.backend_bytes_observed(), packet.len());
        assert_eq!(state.phase(), MysqlConnectionPhase::InitialHandshakeSeen);
        assert_eq!(handshake.server_version, "8.0.36");
        assert_eq!(handshake.connection_id, 0x0102_0304);
        assert_eq!(handshake.capability_flags, Some(0x5678_1234));
        assert_eq!(
            handshake.auth_plugin_name,
            Some("mysql_native_password".to_owned())
        );
        assert!(events.events.is_empty());
    }

    #[test]
    fn mysql_adapter_does_not_observe_initial_handshake_from_client_bytes() {
        let adapter = MysqlProtocolAdapter::new();
        let mut state = adapter.create_connection_state(&test_context());
        let mut events = VecCaptureEventEmitter::default();
        let packet = initial_handshake_packet();

        let observation = adapter
            .observe_client_bytes(state.as_mut(), &packet, &mut events)
            .expect("client bytes should be observed");
        let state = state
            .as_ref()
            .as_any()
            .downcast_ref::<MysqlConnectionState>()
            .expect("state should downcast");

        assert_eq!(observation, ProtocolObservation::new(packet.len(), 0));
        assert_eq!(state.client_bytes_observed(), packet.len());
        assert_eq!(
            state.phase(),
            MysqlConnectionPhase::AwaitingInitialHandshake
        );
        assert!(state.initial_handshake().is_none());
        assert!(events.events.is_empty());
    }

    #[test]
    fn mysql_adapter_keeps_awaiting_phase_for_malformed_backend_handshake() {
        let adapter = MysqlProtocolAdapter::new();
        let mut state = adapter.create_connection_state(&test_context());
        let mut events = VecCaptureEventEmitter::default();

        let observation = adapter
            .observe_backend_bytes(state.as_mut(), &[0x05, 0x00], &mut events)
            .expect("malformed backend bytes should remain non-fatal");
        let state = state
            .as_ref()
            .as_any()
            .downcast_ref::<MysqlConnectionState>()
            .expect("state should downcast");

        assert_eq!(observation, ProtocolObservation::new(2, 0));
        assert_eq!(
            state.phase(),
            MysqlConnectionPhase::AwaitingInitialHandshake
        );
        assert!(state.initial_handshake().is_none());
        assert!(events.events.is_empty());
    }

    #[test]
    fn mysql_adapter_rejects_wrong_connection_state() {
        let adapter = MysqlProtocolAdapter::new();
        let mut state = WrongState;
        let mut events = VecCaptureEventEmitter::default();

        let error = adapter
            .observe_client_bytes(&mut state, b"client", &mut events)
            .expect_err("wrong state should fail");

        assert_eq!(
            error,
            ProtocolAdapterError::InvalidConnectionState {
                expected: "MysqlConnectionState"
            }
        );
    }

    #[derive(Debug)]
    struct WrongState;

    fn test_context() -> ProtocolConnectionContext {
        ProtocolConnectionContext::new(ConnectionInfo {
            id: ConnectionId("conn_1".to_owned()),
            protocol: ProtocolName(MYSQL_PROTOCOL_NAME.to_owned()),
            database_type: DatabaseType("mysql".to_owned()),
            client_addr: "127.0.0.1:51000".to_owned(),
            backend_addr: "127.0.0.1:3306".to_owned(),
            user: None,
            database: None,
            state: ConnectionState::BackendConnected,
            connected_at: Timestamp("connected".to_owned()),
            closed_at: None,
            last_activity_at: None,
            bytes_in: 0,
            bytes_out: 0,
            query_count: 0,
        })
    }

    fn initial_handshake_packet() -> Vec<u8> {
        let payload = representative_handshake_payload();
        let payload_len =
            u32::try_from(payload.len()).expect("test handshake payload should fit u32");
        let mut packet = vec![
            (payload_len & 0xff) as u8,
            ((payload_len >> 8) & 0xff) as u8,
            ((payload_len >> 16) & 0xff) as u8,
            0,
        ];
        packet.extend_from_slice(&payload);

        packet
    }

    fn representative_handshake_payload() -> Vec<u8> {
        let mut payload = Vec::new();

        payload.push(10);
        payload.extend_from_slice(b"8.0.36");
        payload.push(0);
        payload.extend_from_slice(&0x0102_0304u32.to_le_bytes());
        payload.extend_from_slice(b"abcdefgh");
        payload.push(0);
        payload.extend_from_slice(&0x1234u16.to_le_bytes());
        payload.push(0x21);
        payload.extend_from_slice(&0x0002u16.to_le_bytes());
        payload.extend_from_slice(&0x5678u16.to_le_bytes());
        payload.push(21);
        payload.extend_from_slice(&[0; 10]);
        payload.extend_from_slice(b"ijklmnopqrst");
        payload.push(0);
        payload.extend_from_slice(b"mysql_native_password");
        payload.push(0);

        payload
    }
}
