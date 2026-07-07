//! MySQL-compatible protocol adapter for SQL Lens.

mod authentication;
mod command;
mod handshake;
mod packet;

use sql_lens_core::ProtocolName;
use sql_lens_protocol::{
    CaptureEventEmitter, ProtocolAdapter, ProtocolAdapterError, ProtocolConnectionContext,
    ProtocolConnectionState, ProtocolObservation,
};

pub use authentication::{
    MysqlAuthenticationResult, MysqlAuthenticationResultParseError, MysqlAuthenticationStatus,
    parse_authentication_result,
};
pub use command::{
    MYSQL_COM_QUERY, MysqlClientCommand, MysqlComQuery, MysqlCommandKind, MysqlCommandParseError,
    parse_client_command,
};
pub use handshake::{
    MysqlClientHandshakeParseError, MysqlClientHandshakeResponse, MysqlHandshakeParseError,
    MysqlInitialHandshake, parse_client_handshake_response, parse_initial_handshake,
};
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
        state.observe_client_handshake_response(bytes);
        state.observe_client_command(bytes);

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
        state.observe_authentication_result(bytes);

        Ok(ProtocolObservation::new(bytes.len(), 0))
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MysqlConnectionPhase {
    #[default]
    AwaitingInitialHandshake,
    InitialHandshakeSeen,
    ClientHandshakeSeen,
    Authenticated,
    AuthenticationFailed,
}

#[derive(Debug, Default)]
pub struct MysqlConnectionState {
    client_bytes_observed: usize,
    backend_bytes_observed: usize,
    phase: MysqlConnectionPhase,
    initial_handshake: Option<MysqlInitialHandshake>,
    client_handshake: Option<MysqlClientHandshakeResponse>,
    authentication_result: Option<MysqlAuthenticationResult>,
    last_client_command: Option<MysqlClientCommand>,
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

    pub fn client_handshake(&self) -> Option<&MysqlClientHandshakeResponse> {
        self.client_handshake.as_ref()
    }

    pub fn authentication_result(&self) -> Option<&MysqlAuthenticationResult> {
        self.authentication_result.as_ref()
    }

    pub fn last_client_command(&self) -> Option<&MysqlClientCommand> {
        self.last_client_command.as_ref()
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

    fn observe_client_handshake_response(&mut self, bytes: &[u8]) {
        if self.phase != MysqlConnectionPhase::InitialHandshakeSeen {
            return;
        }

        let Ok(packet) = parse_mysql_packet(bytes) else {
            return;
        };

        if packet.header.sequence_id != 1 {
            return;
        }

        let Ok(handshake) = parse_client_handshake_response(packet.payload) else {
            return;
        };

        self.client_handshake = Some(handshake);
        self.phase = MysqlConnectionPhase::ClientHandshakeSeen;
    }

    fn observe_authentication_result(&mut self, bytes: &[u8]) {
        if self.phase != MysqlConnectionPhase::ClientHandshakeSeen {
            return;
        }

        let Ok(packet) = parse_mysql_packet(bytes) else {
            return;
        };

        let Ok(Some(result)) = parse_authentication_result(packet.payload) else {
            return;
        };

        self.phase = match result.status {
            MysqlAuthenticationStatus::Succeeded => MysqlConnectionPhase::Authenticated,
            MysqlAuthenticationStatus::Failed => MysqlConnectionPhase::AuthenticationFailed,
        };
        self.authentication_result = Some(result);
    }

    fn observe_client_command(&mut self, bytes: &[u8]) {
        if self.phase != MysqlConnectionPhase::Authenticated {
            return;
        }

        let Ok(packet) = parse_mysql_packet(bytes) else {
            return;
        };

        let Ok(Some(query)) = parse_client_command(packet.payload) else {
            return;
        };

        self.last_client_command = Some(MysqlClientCommand {
            kind: MysqlCommandKind::Query,
            sequence_id: packet.header.sequence_id,
            sql: query.sql,
        });
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
        assert!(state.client_handshake().is_none());
        assert!(state.authentication_result().is_none());
        assert!(state.last_client_command().is_none());
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
    fn mysql_adapter_observes_client_handshake_response_after_initial_handshake() {
        let adapter = MysqlProtocolAdapter::new();
        let mut state = adapter.create_connection_state(&test_context());
        let mut events = VecCaptureEventEmitter::default();
        let initial_packet = initial_handshake_packet();
        let client_packet = client_handshake_response_packet();

        adapter
            .observe_backend_bytes(state.as_mut(), &initial_packet, &mut events)
            .expect("backend handshake should be observed");
        let observation = adapter
            .observe_client_bytes(state.as_mut(), &client_packet, &mut events)
            .expect("client handshake response should be observed");
        let state = state
            .as_ref()
            .as_any()
            .downcast_ref::<MysqlConnectionState>()
            .expect("state should downcast");
        let client_handshake = state
            .client_handshake()
            .expect("client handshake response should be stored");

        assert_eq!(
            observation,
            ProtocolObservation::new(client_packet.len(), 0)
        );
        assert_eq!(state.phase(), MysqlConnectionPhase::ClientHandshakeSeen);
        assert_eq!(state.client_bytes_observed(), client_packet.len());
        assert_eq!(client_handshake.username, Some("app".to_owned()));
        assert_eq!(client_handshake.database, Some("app_db".to_owned()));
        assert_eq!(
            client_handshake.auth_plugin_name,
            Some("mysql_native_password".to_owned())
        );
        assert!(events.events.is_empty());
    }

    #[test]
    fn mysql_adapter_does_not_observe_client_handshake_before_initial_handshake() {
        let adapter = MysqlProtocolAdapter::new();
        let mut state = adapter.create_connection_state(&test_context());
        let mut events = VecCaptureEventEmitter::default();
        let client_packet = client_handshake_response_packet();

        let observation = adapter
            .observe_client_bytes(state.as_mut(), &client_packet, &mut events)
            .expect("client bytes should be observed");
        let state = state
            .as_ref()
            .as_any()
            .downcast_ref::<MysqlConnectionState>()
            .expect("state should downcast");

        assert_eq!(
            observation,
            ProtocolObservation::new(client_packet.len(), 0)
        );
        assert_eq!(
            state.phase(),
            MysqlConnectionPhase::AwaitingInitialHandshake
        );
        assert!(state.client_handshake().is_none());
        assert!(events.events.is_empty());
    }

    #[test]
    fn mysql_adapter_keeps_initial_handshake_phase_for_malformed_client_response() {
        let adapter = MysqlProtocolAdapter::new();
        let mut state = adapter.create_connection_state(&test_context());
        let mut events = VecCaptureEventEmitter::default();

        adapter
            .observe_backend_bytes(state.as_mut(), &initial_handshake_packet(), &mut events)
            .expect("backend handshake should be observed");
        let observation = adapter
            .observe_client_bytes(state.as_mut(), &[0x05, 0x00], &mut events)
            .expect("malformed client bytes should remain non-fatal");
        let state = state
            .as_ref()
            .as_any()
            .downcast_ref::<MysqlConnectionState>()
            .expect("state should downcast");

        assert_eq!(observation, ProtocolObservation::new(2, 0));
        assert_eq!(state.phase(), MysqlConnectionPhase::InitialHandshakeSeen);
        assert!(state.client_handshake().is_none());
        assert!(events.events.is_empty());
    }

    #[test]
    fn mysql_adapter_observes_successful_authentication_after_client_handshake() {
        let adapter = MysqlProtocolAdapter::new();
        let mut state = adapter.create_connection_state(&test_context());
        let mut events = VecCaptureEventEmitter::default();
        observe_complete_handshake(&adapter, state.as_mut(), &mut events);
        let packet = authentication_ok_packet();

        let observation = adapter
            .observe_backend_bytes(state.as_mut(), &packet, &mut events)
            .expect("authentication result should be observed");
        let state = state
            .as_ref()
            .as_any()
            .downcast_ref::<MysqlConnectionState>()
            .expect("state should downcast");
        let result = state
            .authentication_result()
            .expect("authentication result should be stored");

        assert_eq!(observation, ProtocolObservation::new(packet.len(), 0));
        assert_eq!(state.phase(), MysqlConnectionPhase::Authenticated);
        assert_eq!(result.status, MysqlAuthenticationStatus::Succeeded);
        assert_eq!(result.error_code, None);
        assert_eq!(result.sql_state, None);
        assert_eq!(result.message, None);
        assert!(events.events.is_empty());
    }

    #[test]
    fn mysql_adapter_observes_failed_authentication_after_client_handshake() {
        let adapter = MysqlProtocolAdapter::new();
        let mut state = adapter.create_connection_state(&test_context());
        let mut events = VecCaptureEventEmitter::default();
        observe_complete_handshake(&adapter, state.as_mut(), &mut events);
        let packet = authentication_err_packet();

        let observation = adapter
            .observe_backend_bytes(state.as_mut(), &packet, &mut events)
            .expect("authentication failure should be observed");
        let state = state
            .as_ref()
            .as_any()
            .downcast_ref::<MysqlConnectionState>()
            .expect("state should downcast");
        let result = state
            .authentication_result()
            .expect("authentication failure should be stored");

        assert_eq!(observation, ProtocolObservation::new(packet.len(), 0));
        assert_eq!(state.phase(), MysqlConnectionPhase::AuthenticationFailed);
        assert_eq!(result.status, MysqlAuthenticationStatus::Failed);
        assert_eq!(result.error_code, Some(1045));
        assert_eq!(result.sql_state, Some("28000".to_owned()));
        assert_eq!(result.message, Some("Access denied".to_owned()));
        assert!(events.events.is_empty());
    }

    #[test]
    fn mysql_adapter_does_not_observe_authentication_before_client_handshake() {
        let adapter = MysqlProtocolAdapter::new();
        let mut state = adapter.create_connection_state(&test_context());
        let mut events = VecCaptureEventEmitter::default();
        let packet = authentication_ok_packet();

        let observation = adapter
            .observe_backend_bytes(state.as_mut(), &packet, &mut events)
            .expect("backend bytes should be observed");
        let state = state
            .as_ref()
            .as_any()
            .downcast_ref::<MysqlConnectionState>()
            .expect("state should downcast");

        assert_eq!(observation, ProtocolObservation::new(packet.len(), 0));
        assert_eq!(
            state.phase(),
            MysqlConnectionPhase::AwaitingInitialHandshake
        );
        assert!(state.authentication_result().is_none());
        assert!(events.events.is_empty());
    }

    #[test]
    fn mysql_adapter_keeps_client_handshake_phase_for_unsupported_auth_continuation() {
        let adapter = MysqlProtocolAdapter::new();
        let mut state = adapter.create_connection_state(&test_context());
        let mut events = VecCaptureEventEmitter::default();
        observe_complete_handshake(&adapter, state.as_mut(), &mut events);
        let packet = authentication_continuation_packet();

        let observation = adapter
            .observe_backend_bytes(state.as_mut(), &packet, &mut events)
            .expect("unsupported auth continuation should remain non-fatal");
        let state = state
            .as_ref()
            .as_any()
            .downcast_ref::<MysqlConnectionState>()
            .expect("state should downcast");

        assert_eq!(observation, ProtocolObservation::new(packet.len(), 0));
        assert_eq!(state.phase(), MysqlConnectionPhase::ClientHandshakeSeen);
        assert!(state.authentication_result().is_none());
        assert!(events.events.is_empty());
    }

    #[test]
    fn mysql_adapter_keeps_client_handshake_phase_for_malformed_auth_result() {
        let adapter = MysqlProtocolAdapter::new();
        let mut state = adapter.create_connection_state(&test_context());
        let mut events = VecCaptureEventEmitter::default();
        observe_complete_handshake(&adapter, state.as_mut(), &mut events);
        let packet = packet_with_sequence_id(Vec::new(), 2);

        let observation = adapter
            .observe_backend_bytes(state.as_mut(), &packet, &mut events)
            .expect("malformed auth result should remain non-fatal");
        let state = state
            .as_ref()
            .as_any()
            .downcast_ref::<MysqlConnectionState>()
            .expect("state should downcast");

        assert_eq!(observation, ProtocolObservation::new(packet.len(), 0));
        assert_eq!(state.phase(), MysqlConnectionPhase::ClientHandshakeSeen);
        assert!(state.authentication_result().is_none());
        assert!(events.events.is_empty());
    }

    #[test]
    fn mysql_adapter_does_not_observe_com_query_before_authentication() {
        let adapter = MysqlProtocolAdapter::new();
        let mut state = adapter.create_connection_state(&test_context());
        let mut events = VecCaptureEventEmitter::default();
        observe_complete_handshake(&adapter, state.as_mut(), &mut events);
        let packet = com_query_packet("select 1", 0);

        let observation = adapter
            .observe_client_bytes(state.as_mut(), &packet, &mut events)
            .expect("client command bytes should be observed");
        let state = state
            .as_ref()
            .as_any()
            .downcast_ref::<MysqlConnectionState>()
            .expect("state should downcast");

        assert_eq!(observation, ProtocolObservation::new(packet.len(), 0));
        assert_eq!(state.phase(), MysqlConnectionPhase::ClientHandshakeSeen);
        assert!(state.last_client_command().is_none());
        assert!(events.events.is_empty());
    }

    #[test]
    fn mysql_adapter_observes_com_query_after_authentication() {
        let adapter = MysqlProtocolAdapter::new();
        let mut state = adapter.create_connection_state(&test_context());
        let mut events = VecCaptureEventEmitter::default();
        observe_authenticated_connection(&adapter, state.as_mut(), &mut events);
        let packet = com_query_packet("select * from users", 0);

        let observation = adapter
            .observe_client_bytes(state.as_mut(), &packet, &mut events)
            .expect("COM_QUERY should be observed");
        let state = state
            .as_ref()
            .as_any()
            .downcast_ref::<MysqlConnectionState>()
            .expect("state should downcast");
        let command = state
            .last_client_command()
            .expect("client command should be stored");

        assert_eq!(observation, ProtocolObservation::new(packet.len(), 0));
        assert_eq!(state.phase(), MysqlConnectionPhase::Authenticated);
        assert_eq!(command.kind, MysqlCommandKind::Query);
        assert_eq!(command.sequence_id, 0);
        assert_eq!(command.sql, "select * from users");
        assert!(events.events.is_empty());
    }

    #[test]
    fn mysql_adapter_keeps_authenticated_phase_for_unsupported_command() {
        let adapter = MysqlProtocolAdapter::new();
        let mut state = adapter.create_connection_state(&test_context());
        let mut events = VecCaptureEventEmitter::default();
        observe_authenticated_connection(&adapter, state.as_mut(), &mut events);
        let packet = unsupported_command_packet();

        let observation = adapter
            .observe_client_bytes(state.as_mut(), &packet, &mut events)
            .expect("unsupported command should remain non-fatal");
        let state = state
            .as_ref()
            .as_any()
            .downcast_ref::<MysqlConnectionState>()
            .expect("state should downcast");

        assert_eq!(observation, ProtocolObservation::new(packet.len(), 0));
        assert_eq!(state.phase(), MysqlConnectionPhase::Authenticated);
        assert!(state.last_client_command().is_none());
        assert!(events.events.is_empty());
    }

    #[test]
    fn mysql_adapter_keeps_authenticated_phase_for_invalid_utf8_com_query() {
        let adapter = MysqlProtocolAdapter::new();
        let mut state = adapter.create_connection_state(&test_context());
        let mut events = VecCaptureEventEmitter::default();
        observe_authenticated_connection(&adapter, state.as_mut(), &mut events);
        let packet = invalid_utf8_com_query_packet();

        let observation = adapter
            .observe_client_bytes(state.as_mut(), &packet, &mut events)
            .expect("invalid SQL bytes should remain non-fatal");
        let state = state
            .as_ref()
            .as_any()
            .downcast_ref::<MysqlConnectionState>()
            .expect("state should downcast");

        assert_eq!(observation, ProtocolObservation::new(packet.len(), 0));
        assert_eq!(state.phase(), MysqlConnectionPhase::Authenticated);
        assert!(state.last_client_command().is_none());
        assert!(events.events.is_empty());
    }

    #[test]
    fn mysql_adapter_keeps_authenticated_phase_for_malformed_command() {
        let adapter = MysqlProtocolAdapter::new();
        let mut state = adapter.create_connection_state(&test_context());
        let mut events = VecCaptureEventEmitter::default();
        observe_authenticated_connection(&adapter, state.as_mut(), &mut events);
        let packet = packet_with_sequence_id(Vec::new(), 0);

        let observation = adapter
            .observe_client_bytes(state.as_mut(), &packet, &mut events)
            .expect("malformed command should remain non-fatal");
        let state = state
            .as_ref()
            .as_any()
            .downcast_ref::<MysqlConnectionState>()
            .expect("state should downcast");

        assert_eq!(observation, ProtocolObservation::new(packet.len(), 0));
        assert_eq!(state.phase(), MysqlConnectionPhase::Authenticated);
        assert!(state.last_client_command().is_none());
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
        packet_with_sequence_id(payload, 0)
    }

    fn client_handshake_response_packet() -> Vec<u8> {
        let payload = client_handshake_response_payload();
        packet_with_sequence_id(payload, 1)
    }

    fn authentication_ok_packet() -> Vec<u8> {
        packet_with_sequence_id(vec![0x00], 2)
    }

    fn authentication_err_packet() -> Vec<u8> {
        let mut payload = vec![0xff];
        payload.extend_from_slice(&1045u16.to_le_bytes());
        payload.push(b'#');
        payload.extend_from_slice(b"28000");
        payload.extend_from_slice(b"Access denied");

        packet_with_sequence_id(payload, 2)
    }

    fn authentication_continuation_packet() -> Vec<u8> {
        packet_with_sequence_id(vec![0xfe, b'a', b'u', b't', b'h'], 2)
    }

    fn com_query_packet(sql: &str, sequence_id: u8) -> Vec<u8> {
        let mut payload = vec![MYSQL_COM_QUERY];
        payload.extend_from_slice(sql.as_bytes());

        packet_with_sequence_id(payload, sequence_id)
    }

    fn unsupported_command_packet() -> Vec<u8> {
        packet_with_sequence_id(vec![0x01, b'x'], 0)
    }

    fn invalid_utf8_com_query_packet() -> Vec<u8> {
        packet_with_sequence_id(vec![MYSQL_COM_QUERY, 0xff], 0)
    }

    fn observe_complete_handshake(
        adapter: &MysqlProtocolAdapter,
        state: &mut dyn ProtocolConnectionState,
        events: &mut VecCaptureEventEmitter,
    ) {
        adapter
            .observe_backend_bytes(state, &initial_handshake_packet(), events)
            .expect("backend handshake should be observed");
        adapter
            .observe_client_bytes(state, &client_handshake_response_packet(), events)
            .expect("client handshake response should be observed");
    }

    fn observe_authenticated_connection(
        adapter: &MysqlProtocolAdapter,
        state: &mut dyn ProtocolConnectionState,
        events: &mut VecCaptureEventEmitter,
    ) {
        observe_complete_handshake(adapter, state, events);
        adapter
            .observe_backend_bytes(state, &authentication_ok_packet(), events)
            .expect("authentication OK should be observed");
    }

    fn packet_with_sequence_id(payload: Vec<u8>, sequence_id: u8) -> Vec<u8> {
        let payload_len =
            u32::try_from(payload.len()).expect("test handshake payload should fit u32");
        let mut packet = vec![
            (payload_len & 0xff) as u8,
            ((payload_len >> 8) & 0xff) as u8,
            ((payload_len >> 16) & 0xff) as u8,
            sequence_id,
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

    fn client_handshake_response_payload() -> Vec<u8> {
        const CLIENT_CONNECT_WITH_DB: u32 = 0x0000_0008;
        const CLIENT_PROTOCOL_41: u32 = 0x0000_0200;
        const CLIENT_SECURE_CONNECTION: u32 = 0x0000_8000;
        const CLIENT_PLUGIN_AUTH: u32 = 0x0008_0000;
        const CLIENT_HANDSHAKE_RESERVED_LEN: usize = 23;

        let capability_flags = CLIENT_PROTOCOL_41
            | CLIENT_SECURE_CONNECTION
            | CLIENT_CONNECT_WITH_DB
            | CLIENT_PLUGIN_AUTH;
        let auth_response = b"secret-password";
        let mut payload = Vec::new();

        payload.extend_from_slice(&capability_flags.to_le_bytes());
        payload.extend_from_slice(&(16 * 1024 * 1024u32).to_le_bytes());
        payload.push(0x21);
        payload.extend_from_slice(&[0; CLIENT_HANDSHAKE_RESERVED_LEN]);
        payload.extend_from_slice(b"app");
        payload.push(0);
        payload.push(
            u8::try_from(auth_response.len()).expect("test auth response length should fit u8"),
        );
        payload.extend_from_slice(auth_response);
        payload.extend_from_slice(b"app_db");
        payload.push(0);
        payload.extend_from_slice(b"mysql_native_password");
        payload.push(0);

        payload
    }
}
