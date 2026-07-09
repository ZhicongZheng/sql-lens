use super::*;
use sql_lens_core::{
    CaptureStatus, ConnectionId, ConnectionInfo, ConnectionState, DatabaseType, DurationMillis,
    ProtocolMetadata, ProtocolName, QueryTiming, SqlEvent, SqlEventId, SqlEventKind, Timestamp,
};

#[derive(Debug)]
struct DummyAdapter {
    protocol: &'static str,
}

impl DummyAdapter {
    fn new(protocol: &'static str) -> Self {
        Self { protocol }
    }

    fn state_mut<'a>(
        &self,
        state: &'a mut dyn ProtocolConnectionState,
    ) -> Result<&'a mut DummyState, ProtocolAdapterError> {
        state.as_any_mut().downcast_mut::<DummyState>().ok_or(
            ProtocolAdapterError::InvalidConnectionState {
                expected: "DummyState",
            },
        )
    }
}

impl ProtocolAdapter for DummyAdapter {
    fn protocol_name(&self) -> ProtocolName {
        ProtocolName(self.protocol.to_owned())
    }

    fn create_connection_state(
        &self,
        _context: &ProtocolConnectionContext,
    ) -> Box<dyn ProtocolConnectionState> {
        Box::new(DummyState::default())
    }

    fn observe_client_bytes(
        &self,
        state: &mut dyn ProtocolConnectionState,
        bytes: &[u8],
        events: &mut dyn CaptureEventEmitter,
    ) -> Result<ProtocolObservation, ProtocolAdapterError> {
        let state = self.state_mut(state)?;
        state.client_bytes += bytes.len();

        let events_emitted = if bytes.is_empty() {
            0
        } else {
            events.emit(test_event("evt_client"));
            1
        };

        Ok(ProtocolObservation::new(bytes.len(), events_emitted))
    }

    fn observe_backend_bytes(
        &self,
        state: &mut dyn ProtocolConnectionState,
        bytes: &[u8],
        _events: &mut dyn CaptureEventEmitter,
    ) -> Result<ProtocolObservation, ProtocolAdapterError> {
        let state = self.state_mut(state)?;
        state.backend_bytes += bytes.len();

        Ok(ProtocolObservation::new(bytes.len(), 0))
    }
}

#[derive(Debug, Default)]
struct DummyState {
    client_bytes: usize,
    backend_bytes: usize,
}

#[derive(Debug, Default)]
struct VecCaptureEventEmitter {
    events: Vec<SqlEvent>,
}

impl CaptureEventEmitter for VecCaptureEventEmitter {
    fn emit(&mut self, event: SqlEvent) {
        self.events.push(event);
    }
}

fn test_context() -> ProtocolConnectionContext {
    ProtocolConnectionContext::new(ConnectionInfo {
        id: ConnectionId("conn_1".to_owned()),
        target_name: Some("mysql-local".to_owned()),
        protocol: ProtocolName("dummy".to_owned()),
        database_type: DatabaseType("dummy".to_owned()),
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

fn test_event(id: &str) -> SqlEvent {
    SqlEvent {
        id: SqlEventId(id.to_owned()),
        timestamp: Timestamp("2026-07-06T09:00:00Z".to_owned()),
        target_name: Some("mysql-local".to_owned()),
        protocol: ProtocolName("dummy".to_owned()),
        database_type: DatabaseType("dummy".to_owned()),
        connection_id: ConnectionId("conn_1".to_owned()),
        client_addr: "127.0.0.1:51000".to_owned(),
        backend_addr: "127.0.0.1:3306".to_owned(),
        user: None,
        database: None,
        kind: SqlEventKind::Query,
        status: CaptureStatus::Ok,
        duration: DurationMillis(1),
        original_sql: "SELECT 1".to_owned(),
        normalized_sql: Some("select 1".to_owned()),
        expanded_sql: None,
        fingerprint: Some("select ?".to_owned()),
        parameters: Vec::new(),
        result: None,
        error: None,
        timings: QueryTiming {
            started_at: Timestamp("2026-07-06T09:00:00Z".to_owned()),
            ended_at: Some(Timestamp("2026-07-06T09:00:00Z".to_owned())),
            duration: DurationMillis(1),
        },
        metadata: ProtocolMetadata {
            protocol: ProtocolName("dummy".to_owned()),
            fields: Vec::new(),
        },
    }
}

#[test]
fn adapter_observes_client_bytes() {
    let adapter = DummyAdapter::new("dummy");
    let context = test_context();
    let mut state = adapter.create_connection_state(&context);
    let mut events = VecCaptureEventEmitter::default();

    let observation = adapter
        .observe_client_bytes(state.as_mut(), b"client bytes", &mut events)
        .expect("client bytes should be observed");
    let state = state
        .as_ref()
        .as_any()
        .downcast_ref::<DummyState>()
        .expect("state should downcast");

    assert_eq!(observation.bytes_observed, 12);
    assert_eq!(observation.events_emitted, 1);
    assert_eq!(state.client_bytes, 12);
}

#[test]
fn adapter_observes_backend_bytes() {
    let adapter = DummyAdapter::new("dummy");
    let context = test_context();
    let mut state = adapter.create_connection_state(&context);
    let mut events = VecCaptureEventEmitter::default();

    let observation = adapter
        .observe_backend_bytes(state.as_mut(), b"ok", &mut events)
        .expect("backend bytes should be observed");
    let state = state
        .as_ref()
        .as_any()
        .downcast_ref::<DummyState>()
        .expect("state should downcast");

    assert_eq!(observation.bytes_observed, 2);
    assert_eq!(observation.events_emitted, 0);
    assert_eq!(state.backend_bytes, 2);
}

#[test]
fn adapter_emits_capture_events() {
    let adapter = DummyAdapter::new("dummy");
    let context = test_context();
    let mut state = adapter.create_connection_state(&context);
    let mut events = VecCaptureEventEmitter::default();

    adapter
        .observe_client_bytes(state.as_mut(), b"select 1", &mut events)
        .expect("client bytes should be observed");

    assert_eq!(events.events.len(), 1);
    assert_eq!(events.events[0].id, SqlEventId("evt_client".to_owned()));
}

#[test]
fn adapter_supports_protocol_specific_state_downcast() {
    let adapter = DummyAdapter::new("dummy");
    let context = test_context();
    let state = adapter.create_connection_state(&context);

    assert!(state.as_ref().as_any().is::<DummyState>());
}

#[test]
fn protocol_adapter_is_object_safe() {
    let adapter: Box<dyn ProtocolAdapter> = Box::new(DummyAdapter::new("dummy"));
    let context = test_context();
    let mut state = adapter.create_connection_state(&context);
    let mut events = VecCaptureEventEmitter::default();

    let observation = adapter
        .observe_client_bytes(state.as_mut(), b"abc", &mut events)
        .expect("trait object should observe bytes");

    assert_eq!(adapter.protocol_name(), ProtocolName("dummy".to_owned()));
    assert_eq!(observation, ProtocolObservation::new(3, 1));
}

#[test]
fn registry_registers_and_resolves_adapter() {
    let mut registry = ProtocolAdapterRegistry::new();

    registry
        .register(DummyAdapter::new("dummy"))
        .expect("adapter should register");

    let adapter = registry
        .resolve(&ProtocolName("dummy".to_owned()))
        .expect("adapter should resolve");

    assert_eq!(registry.len(), 1);
    assert!(!registry.is_empty());
    assert_eq!(adapter.protocol_name(), ProtocolName("dummy".to_owned()));
}

#[test]
fn registry_reports_adapter_presence() {
    let mut registry = ProtocolAdapterRegistry::new();

    assert!(!registry.contains(&ProtocolName("dummy".to_owned())));

    registry
        .register(DummyAdapter::new("dummy"))
        .expect("adapter should register");

    assert!(registry.contains(&ProtocolName("dummy".to_owned())));
    assert!(!registry.contains(&ProtocolName("missing".to_owned())));
}

#[test]
fn registry_returns_unknown_adapter_error() {
    let registry = ProtocolAdapterRegistry::new();

    let error = registry
        .resolve(&ProtocolName("missing".to_owned()))
        .expect_err("missing adapter should fail");

    assert_eq!(
        error,
        ProtocolAdapterRegistryError::UnknownAdapter {
            protocol: ProtocolName("missing".to_owned()),
        }
    );
    assert!(!error.to_string().is_empty());
}

#[test]
fn registry_rejects_duplicate_adapter_names() {
    let mut registry = ProtocolAdapterRegistry::new();

    registry
        .register(DummyAdapter::new("dummy"))
        .expect("first adapter should register");
    let error = registry
        .register(DummyAdapter::new("dummy"))
        .expect_err("duplicate adapter should fail");

    assert_eq!(
        error,
        ProtocolAdapterRegistryError::DuplicateAdapter {
            protocol: ProtocolName("dummy".to_owned()),
        }
    );
}

#[test]
fn adapter_error_supports_standard_error_traits() {
    let error = ProtocolAdapterError::InvalidConnectionState {
        expected: "DummyState",
    };

    assert!(!error.to_string().is_empty());
    assert!(std::error::Error::source(&error).is_none());
}
