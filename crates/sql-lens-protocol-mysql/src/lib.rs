//! MySQL-compatible protocol adapter for SQL Lens.

mod packet;

use sql_lens_core::ProtocolName;
use sql_lens_protocol::{
    CaptureEventEmitter, ProtocolAdapter, ProtocolAdapterError, ProtocolConnectionContext,
    ProtocolConnectionState, ProtocolObservation,
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

        Ok(ProtocolObservation::new(bytes.len(), 0))
    }
}

#[derive(Debug, Default)]
pub struct MysqlConnectionState {
    client_bytes_observed: usize,
    backend_bytes_observed: usize,
}

impl MysqlConnectionState {
    pub fn client_bytes_observed(&self) -> usize {
        self.client_bytes_observed
    }

    pub fn backend_bytes_observed(&self) -> usize {
        self.backend_bytes_observed
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
}
