//! MySQL-compatible protocol adapter for SQL Lens.

mod authentication;
mod command;
mod err;
mod handshake;
mod ok;
mod packet;
mod prepare;

use std::{
    sync::Arc,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use sql_lens_core::{
    CaptureStatus, ConnectionInfo, DurationMillis, ErrorSummary, MetadataField, MetadataValue,
    ProtocolMetadata, ProtocolName, QueryTiming, ResultSummary, SqlEvent, SqlEventId, SqlEventKind,
    Timestamp,
};
use sql_lens_protocol::{
    CaptureEventEmitter, ProtocolAdapter, ProtocolAdapterError, ProtocolConnectionContext,
    ProtocolConnectionState, ProtocolObservation,
};

pub use authentication::{
    MysqlAuthenticationResult, MysqlAuthenticationResultParseError, MysqlAuthenticationStatus,
    parse_authentication_result,
};
pub use command::{
    MYSQL_COM_QUERY, MYSQL_COM_STMT_PREPARE, MysqlClientCommand, MysqlComQuery,
    MysqlComStmtPrepare, MysqlCommandKind, MysqlCommandParseError, MysqlParsedClientCommand,
    parse_client_command,
};
pub use err::{MysqlErrPacketParseError, MysqlErrPacketSummary, parse_err_packet_summary};
pub use handshake::{
    MysqlClientHandshakeParseError, MysqlClientHandshakeResponse, MysqlHandshakeParseError,
    MysqlInitialHandshake, parse_client_handshake_response, parse_initial_handshake,
};
pub use ok::{MysqlOkPacketParseError, MysqlOkPacketSummary, parse_ok_packet_summary};
pub use packet::{
    MYSQL_PACKET_HEADER_LEN, MysqlPacket, MysqlPacketHeader, MysqlPacketParseError,
    parse_mysql_packet,
};
pub use prepare::{
    MysqlComStmtPrepareOk, MysqlComStmtPrepareResponse, MysqlComStmtPrepareResponseParseError,
    parse_com_stmt_prepare_response,
};

pub const MYSQL_PROTOCOL_NAME: &str = "mysql";

#[derive(Debug, Clone)]
pub struct MysqlProtocolAdapter {
    clock: Arc<dyn MysqlObservationClock>,
}

impl MysqlProtocolAdapter {
    pub fn new() -> Self {
        Self::with_clock(Arc::new(SystemMysqlObservationClock))
    }

    pub fn with_clock(clock: Arc<dyn MysqlObservationClock>) -> Self {
        Self { clock }
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

impl Default for MysqlProtocolAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl ProtocolAdapter for MysqlProtocolAdapter {
    fn protocol_name(&self) -> ProtocolName {
        ProtocolName(MYSQL_PROTOCOL_NAME.to_owned())
    }

    fn create_connection_state(
        &self,
        context: &ProtocolConnectionContext,
    ) -> Box<dyn ProtocolConnectionState> {
        Box::new(MysqlConnectionState::new(context.connection.clone()))
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
        state.observe_client_command(bytes, self.clock.as_ref());

        Ok(ProtocolObservation::new(bytes.len(), 0))
    }

    fn observe_backend_bytes(
        &self,
        state: &mut dyn ProtocolConnectionState,
        bytes: &[u8],
        events: &mut dyn CaptureEventEmitter,
    ) -> Result<ProtocolObservation, ProtocolAdapterError> {
        let state = self.state_mut(state)?;
        state.backend_bytes_observed += bytes.len();
        state.observe_initial_handshake(bytes);
        state.observe_authentication_result(bytes);
        let prepare_response_consumed = state.observe_backend_statement_prepare_response(bytes);
        let events_emitted = if prepare_response_consumed {
            0
        } else {
            state.observe_backend_query_response(bytes, events, self.clock.as_ref())
        };

        Ok(ProtocolObservation::new(bytes.len(), events_emitted))
    }
}

#[derive(Debug, Clone)]
pub struct MysqlObservationTime {
    pub timestamp: Timestamp,
    pub monotonic: Instant,
}

pub trait MysqlObservationClock: std::fmt::Debug + Send + Sync {
    fn now(&self) -> MysqlObservationTime;
}

#[derive(Debug, Default)]
struct SystemMysqlObservationClock;

impl MysqlObservationClock for SystemMysqlObservationClock {
    fn now(&self) -> MysqlObservationTime {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or_default();

        MysqlObservationTime {
            timestamp: Timestamp(format!("unix_ms:{millis}")),
            monotonic: Instant::now(),
        }
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MysqlPendingQuery {
    pub command: MysqlClientCommand,
    pub started_at: Timestamp,
    pub started_monotonic: Instant,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MysqlPendingStatementPrepare {
    pub command: MysqlClientCommand,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MysqlStatementPrepareOutcome {
    pub command: MysqlClientCommand,
    pub response_sequence_id: u8,
    pub response: MysqlStatementPrepareResponseState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MysqlStatementPrepareResponseState {
    Prepared {
        statement_id: u32,
        num_columns: u16,
        num_params: u16,
        warning_count: Option<u16>,
    },
    Failed {
        error: MysqlErrPacketSummary,
    },
}

#[derive(Debug)]
pub struct MysqlConnectionState {
    client_bytes_observed: usize,
    backend_bytes_observed: usize,
    connection: ConnectionInfo,
    phase: MysqlConnectionPhase,
    initial_handshake: Option<MysqlInitialHandshake>,
    client_handshake: Option<MysqlClientHandshakeResponse>,
    authentication_result: Option<MysqlAuthenticationResult>,
    last_client_command: Option<MysqlClientCommand>,
    pending_query: Option<MysqlPendingQuery>,
    pending_statement_prepare: Option<MysqlPendingStatementPrepare>,
    last_statement_prepare_outcome: Option<MysqlStatementPrepareOutcome>,
    next_query_sequence: u64,
}

impl MysqlConnectionState {
    pub fn new(connection: ConnectionInfo) -> Self {
        Self {
            client_bytes_observed: 0,
            backend_bytes_observed: 0,
            connection,
            phase: MysqlConnectionPhase::AwaitingInitialHandshake,
            initial_handshake: None,
            client_handshake: None,
            authentication_result: None,
            last_client_command: None,
            pending_query: None,
            pending_statement_prepare: None,
            last_statement_prepare_outcome: None,
            next_query_sequence: 1,
        }
    }

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

    pub fn pending_query(&self) -> Option<&MysqlPendingQuery> {
        self.pending_query.as_ref()
    }

    pub fn pending_statement_prepare(&self) -> Option<&MysqlPendingStatementPrepare> {
        self.pending_statement_prepare.as_ref()
    }

    pub fn last_statement_prepare_outcome(&self) -> Option<&MysqlStatementPrepareOutcome> {
        self.last_statement_prepare_outcome.as_ref()
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

    fn observe_client_command(&mut self, bytes: &[u8], clock: &dyn MysqlObservationClock) {
        if self.phase != MysqlConnectionPhase::Authenticated {
            return;
        }

        let Ok(packet) = parse_mysql_packet(bytes) else {
            return;
        };

        let Ok(Some(command)) = parse_client_command(packet.payload) else {
            return;
        };

        match command {
            MysqlParsedClientCommand::Query(query) => {
                let client_command = MysqlClientCommand {
                    kind: MysqlCommandKind::Query,
                    sequence_id: packet.header.sequence_id,
                    sql: query.sql,
                };
                let time = clock.now();

                self.last_client_command = Some(client_command.clone());
                self.pending_query = Some(MysqlPendingQuery {
                    command: client_command,
                    started_at: time.timestamp,
                    started_monotonic: time.monotonic,
                });
            }
            MysqlParsedClientCommand::StatementPrepare(prepare) => {
                let client_command = MysqlClientCommand {
                    kind: MysqlCommandKind::StatementPrepare,
                    sequence_id: packet.header.sequence_id,
                    sql: prepare.template_sql,
                };

                self.last_client_command = Some(client_command.clone());
                self.pending_statement_prepare = Some(MysqlPendingStatementPrepare {
                    command: client_command,
                });
            }
        }
    }

    fn observe_backend_query_response(
        &mut self,
        bytes: &[u8],
        events: &mut dyn CaptureEventEmitter,
        clock: &dyn MysqlObservationClock,
    ) -> usize {
        if self.phase != MysqlConnectionPhase::Authenticated || self.pending_query.is_none() {
            return 0;
        }

        let Ok(packet) = parse_mysql_packet(bytes) else {
            return 0;
        };

        let Some(status) = query_terminal_status(packet.payload) else {
            return 0;
        };
        let ok_summary = if status == CaptureStatus::Ok {
            parse_ok_packet_summary(packet.payload).ok().flatten()
        } else {
            None
        };
        let err_summary = if status == CaptureStatus::Error {
            parse_err_packet_summary(packet.payload).ok().flatten()
        } else {
            None
        };

        let Some(pending) = self.pending_query.take() else {
            return 0;
        };
        let ended = clock.now();
        let event = self.query_event(pending, ended, status, ok_summary, err_summary);

        events.emit(event);

        1
    }

    fn observe_backend_statement_prepare_response(&mut self, bytes: &[u8]) -> bool {
        if self.phase != MysqlConnectionPhase::Authenticated
            || self.pending_statement_prepare.is_none()
        {
            return false;
        }

        let Ok(packet) = parse_mysql_packet(bytes) else {
            return false;
        };

        let Ok(Some(response)) = parse_com_stmt_prepare_response(packet.payload) else {
            return false;
        };

        let Some(pending) = self.pending_statement_prepare.take() else {
            return false;
        };

        self.last_statement_prepare_outcome = Some(MysqlStatementPrepareOutcome {
            command: pending.command,
            response_sequence_id: packet.header.sequence_id,
            response: match response {
                MysqlComStmtPrepareResponse::Ok(ok) => {
                    MysqlStatementPrepareResponseState::Prepared {
                        statement_id: ok.statement_id,
                        num_columns: ok.num_columns,
                        num_params: ok.num_params,
                        warning_count: ok.warning_count,
                    }
                }
                MysqlComStmtPrepareResponse::Error(error) => {
                    MysqlStatementPrepareResponseState::Failed { error }
                }
            },
        });

        true
    }

    fn query_event(
        &mut self,
        pending: MysqlPendingQuery,
        ended: MysqlObservationTime,
        status: CaptureStatus,
        ok_summary: Option<MysqlOkPacketSummary>,
        err_summary: Option<MysqlErrPacketSummary>,
    ) -> SqlEvent {
        let duration = duration_millis(pending.started_monotonic, ended.monotonic);
        let event_id = SqlEventId(format!(
            "{}_query_{}",
            self.connection.id.0, self.next_query_sequence
        ));
        self.next_query_sequence += 1;
        let command_sequence_id = pending.command.sequence_id;
        let original_sql = pending.command.sql;
        let result = ok_summary.map(|summary| ResultSummary {
            affected_rows: Some(summary.affected_rows),
            returned_rows: None,
        });
        let error = err_summary.map(|summary| ErrorSummary {
            code: Some(summary.error_code.to_string()),
            sql_state: summary.sql_state,
            message: summary.message,
            metadata: Some(ProtocolMetadata {
                protocol: ProtocolName(MYSQL_PROTOCOL_NAME.to_owned()),
                fields: vec![MetadataField {
                    key: "mysql_error_code".to_owned(),
                    value: MetadataValue::Unsigned(u64::from(summary.error_code)),
                }],
            }),
        });
        let mut metadata_fields = vec![
            MetadataField {
                key: "command".to_owned(),
                value: MetadataValue::String("COM_QUERY".to_owned()),
            },
            MetadataField {
                key: "command_sequence_id".to_owned(),
                value: MetadataValue::Unsigned(u64::from(command_sequence_id)),
            },
        ];

        if let Some(status_flags) = ok_summary.and_then(|summary| summary.status_flags) {
            metadata_fields.push(MetadataField {
                key: "ok_status_flags".to_owned(),
                value: MetadataValue::Unsigned(u64::from(status_flags)),
            });
        }

        SqlEvent {
            id: event_id,
            timestamp: pending.started_at.clone(),
            protocol: self.connection.protocol.clone(),
            database_type: self.connection.database_type.clone(),
            connection_id: self.connection.id.clone(),
            client_addr: self.connection.client_addr.clone(),
            backend_addr: self.connection.backend_addr.clone(),
            user: self.connection.user.clone(),
            database: self.connection.database.clone(),
            kind: SqlEventKind::Query,
            status,
            duration,
            original_sql,
            normalized_sql: None,
            expanded_sql: None,
            fingerprint: None,
            parameters: Vec::new(),
            result,
            error,
            timings: QueryTiming {
                started_at: pending.started_at,
                ended_at: Some(ended.timestamp),
                duration,
            },
            metadata: ProtocolMetadata {
                protocol: ProtocolName(MYSQL_PROTOCOL_NAME.to_owned()),
                fields: metadata_fields,
            },
        }
    }
}

fn query_terminal_status(payload: &[u8]) -> Option<CaptureStatus> {
    match payload.first() {
        Some(0x00) => Some(CaptureStatus::Ok),
        Some(0xff) => Some(CaptureStatus::Error),
        _ => None,
    }
}

fn duration_millis(started: Instant, ended: Instant) -> DurationMillis {
    let millis = ended.saturating_duration_since(started).as_millis();

    DurationMillis(u64::try_from(millis).unwrap_or(u64::MAX))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::VecDeque, sync::Mutex, time::Duration};

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
        assert!(state.pending_query().is_none());
        assert!(state.pending_statement_prepare().is_none());
        assert!(state.last_statement_prepare_outcome().is_none());
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
        assert!(state.pending_statement_prepare().is_none());
        assert!(events.events.is_empty());
    }

    #[test]
    fn mysql_adapter_does_not_observe_com_stmt_prepare_before_authentication() {
        let adapter = MysqlProtocolAdapter::new();
        let mut state = adapter.create_connection_state(&test_context());
        let mut events = VecCaptureEventEmitter::default();
        observe_complete_handshake(&adapter, state.as_mut(), &mut events);
        let packet = com_stmt_prepare_packet("select * from users where id = ?", 0);

        let observation = adapter
            .observe_client_bytes(state.as_mut(), &packet, &mut events)
            .expect("client prepare bytes should be observed");
        let state = state
            .as_ref()
            .as_any()
            .downcast_ref::<MysqlConnectionState>()
            .expect("state should downcast");

        assert_eq!(observation, ProtocolObservation::new(packet.len(), 0));
        assert_eq!(state.phase(), MysqlConnectionPhase::ClientHandshakeSeen);
        assert!(state.last_client_command().is_none());
        assert!(state.pending_statement_prepare().is_none());
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
        assert!(state.pending_query().is_some());
        assert!(state.pending_statement_prepare().is_none());
        assert!(events.events.is_empty());
    }

    #[test]
    fn mysql_adapter_observes_com_stmt_prepare_after_authentication() {
        let adapter = MysqlProtocolAdapter::with_clock(manual_clock(&[]));
        let mut state = adapter.create_connection_state(&test_context());
        let mut events = VecCaptureEventEmitter::default();
        observe_authenticated_connection(&adapter, state.as_mut(), &mut events);
        let packet = com_stmt_prepare_packet("select * from users where id = ?", 7);

        let observation = adapter
            .observe_client_bytes(state.as_mut(), &packet, &mut events)
            .expect("COM_STMT_PREPARE should be observed");
        let state = state
            .as_ref()
            .as_any()
            .downcast_ref::<MysqlConnectionState>()
            .expect("state should downcast");
        let command = state
            .last_client_command()
            .expect("client command should be stored");
        let pending = state
            .pending_statement_prepare()
            .expect("pending statement prepare should be stored");

        assert_eq!(observation, ProtocolObservation::new(packet.len(), 0));
        assert_eq!(state.phase(), MysqlConnectionPhase::Authenticated);
        assert_eq!(command.kind, MysqlCommandKind::StatementPrepare);
        assert_eq!(command.sequence_id, 7);
        assert_eq!(command.sql, "select * from users where id = ?");
        assert_eq!(&pending.command, command);
        assert!(state.pending_query().is_none());
        assert!(events.events.is_empty());
    }

    #[test]
    fn mysql_adapter_consumes_pending_prepare_after_ok_response() {
        let adapter = MysqlProtocolAdapter::with_clock(manual_clock(&[]));
        let mut state = adapter.create_connection_state(&test_context());
        let mut events = VecCaptureEventEmitter::default();
        observe_authenticated_connection(&adapter, state.as_mut(), &mut events);

        adapter
            .observe_client_bytes(
                state.as_mut(),
                &com_stmt_prepare_packet("select * from users where id = ?", 0),
                &mut events,
            )
            .expect("COM_STMT_PREPARE should start pending prepare");
        let response = prepare_ok_packet(0x1122_3344, 3, 2, Some(7), 1);
        let observation = adapter
            .observe_backend_bytes(state.as_mut(), &response, &mut events)
            .expect("prepare OK should be observed");
        let state = state
            .as_ref()
            .as_any()
            .downcast_ref::<MysqlConnectionState>()
            .expect("state should downcast");
        let outcome = state
            .last_statement_prepare_outcome()
            .expect("prepare outcome should be stored");

        assert_eq!(observation, ProtocolObservation::new(response.len(), 0));
        assert!(state.pending_statement_prepare().is_none());
        assert_eq!(outcome.command.kind, MysqlCommandKind::StatementPrepare);
        assert_eq!(outcome.command.sequence_id, 0);
        assert_eq!(outcome.command.sql, "select * from users where id = ?");
        assert_eq!(outcome.response_sequence_id, 1);
        assert_eq!(
            outcome.response,
            MysqlStatementPrepareResponseState::Prepared {
                statement_id: 0x1122_3344,
                num_columns: 3,
                num_params: 2,
                warning_count: Some(7),
            }
        );
        assert!(events.events.is_empty());
    }

    #[test]
    fn mysql_adapter_consumes_pending_prepare_after_err_response() {
        let adapter = MysqlProtocolAdapter::with_clock(manual_clock(&[]));
        let mut state = adapter.create_connection_state(&test_context());
        let mut events = VecCaptureEventEmitter::default();
        observe_authenticated_connection(&adapter, state.as_mut(), &mut events);

        adapter
            .observe_client_bytes(
                state.as_mut(),
                &com_stmt_prepare_packet("select bad", 0),
                &mut events,
            )
            .expect("COM_STMT_PREPARE should start pending prepare");
        let response = prepare_err_packet(1);
        let observation = adapter
            .observe_backend_bytes(state.as_mut(), &response, &mut events)
            .expect("prepare ERR should be observed");
        let state = state
            .as_ref()
            .as_any()
            .downcast_ref::<MysqlConnectionState>()
            .expect("state should downcast");
        let outcome = state
            .last_statement_prepare_outcome()
            .expect("prepare outcome should be stored");

        assert_eq!(observation, ProtocolObservation::new(response.len(), 0));
        assert!(state.pending_statement_prepare().is_none());
        assert_eq!(outcome.command.kind, MysqlCommandKind::StatementPrepare);
        assert_eq!(outcome.command.sql, "select bad");
        assert_eq!(outcome.response_sequence_id, 1);
        assert_eq!(
            outcome.response,
            MysqlStatementPrepareResponseState::Failed {
                error: MysqlErrPacketSummary {
                    error_code: 1064,
                    sql_state: Some("42000".to_owned()),
                    message: "You have an error".to_owned(),
                },
            }
        );
        assert!(events.events.is_empty());
    }

    #[test]
    fn mysql_adapter_ignores_prepare_response_without_pending_prepare() {
        let adapter = MysqlProtocolAdapter::with_clock(manual_clock(&[]));
        let mut state = adapter.create_connection_state(&test_context());
        let mut events = VecCaptureEventEmitter::default();
        observe_authenticated_connection(&adapter, state.as_mut(), &mut events);
        let response = prepare_ok_packet(42, 0, 1, None, 1);

        let observation = adapter
            .observe_backend_bytes(state.as_mut(), &response, &mut events)
            .expect("prepare response without pending state should be non-fatal");
        let state = state
            .as_ref()
            .as_any()
            .downcast_ref::<MysqlConnectionState>()
            .expect("state should downcast");

        assert_eq!(observation, ProtocolObservation::new(response.len(), 0));
        assert!(state.pending_statement_prepare().is_none());
        assert!(state.last_statement_prepare_outcome().is_none());
        assert!(events.events.is_empty());
    }

    #[test]
    fn mysql_adapter_keeps_pending_prepare_for_malformed_response() {
        let adapter = MysqlProtocolAdapter::with_clock(manual_clock(&[]));
        let mut state = adapter.create_connection_state(&test_context());
        let mut events = VecCaptureEventEmitter::default();
        observe_authenticated_connection(&adapter, state.as_mut(), &mut events);

        adapter
            .observe_client_bytes(
                state.as_mut(),
                &com_stmt_prepare_packet("select * from users where id = ?", 0),
                &mut events,
            )
            .expect("COM_STMT_PREPARE should start pending prepare");
        let response = malformed_prepare_ok_packet();
        let observation = adapter
            .observe_backend_bytes(state.as_mut(), &response, &mut events)
            .expect("malformed prepare response should remain non-fatal");
        let state = state
            .as_ref()
            .as_any()
            .downcast_ref::<MysqlConnectionState>()
            .expect("state should downcast");
        let pending = state
            .pending_statement_prepare()
            .expect("pending prepare should remain stored");

        assert_eq!(observation, ProtocolObservation::new(response.len(), 0));
        assert_eq!(pending.command.sql, "select * from users where id = ?");
        assert!(state.last_statement_prepare_outcome().is_none());
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
        assert!(state.pending_statement_prepare().is_none());
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
        assert!(state.pending_statement_prepare().is_none());
        assert!(events.events.is_empty());
    }

    #[test]
    fn mysql_adapter_keeps_authenticated_phase_for_invalid_utf8_com_stmt_prepare() {
        let adapter = MysqlProtocolAdapter::new();
        let mut state = adapter.create_connection_state(&test_context());
        let mut events = VecCaptureEventEmitter::default();
        observe_authenticated_connection(&adapter, state.as_mut(), &mut events);
        let packet = invalid_utf8_com_stmt_prepare_packet();

        let observation = adapter
            .observe_client_bytes(state.as_mut(), &packet, &mut events)
            .expect("invalid template SQL bytes should remain non-fatal");
        let state = state
            .as_ref()
            .as_any()
            .downcast_ref::<MysqlConnectionState>()
            .expect("state should downcast");

        assert_eq!(observation, ProtocolObservation::new(packet.len(), 0));
        assert_eq!(state.phase(), MysqlConnectionPhase::Authenticated);
        assert!(state.last_client_command().is_none());
        assert!(state.pending_statement_prepare().is_none());
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
        assert!(state.pending_statement_prepare().is_none());
        assert!(events.events.is_empty());
    }

    #[test]
    fn mysql_adapter_starts_pending_query_for_com_query_after_authentication() {
        let adapter = MysqlProtocolAdapter::with_clock(manual_clock(&[(0, "query_start")]));
        let mut state = adapter.create_connection_state(&test_context());
        let mut events = VecCaptureEventEmitter::default();
        observe_authenticated_connection(&adapter, state.as_mut(), &mut events);
        let packet = com_query_packet("select 1", 0);

        let observation = adapter
            .observe_client_bytes(state.as_mut(), &packet, &mut events)
            .expect("COM_QUERY should start pending query");
        let state = state
            .as_ref()
            .as_any()
            .downcast_ref::<MysqlConnectionState>()
            .expect("state should downcast");
        let pending = state
            .pending_query()
            .expect("pending query should be stored");

        assert_eq!(observation, ProtocolObservation::new(packet.len(), 0));
        assert_eq!(pending.command.kind, MysqlCommandKind::Query);
        assert_eq!(pending.command.sequence_id, 0);
        assert_eq!(pending.command.sql, "select 1");
        assert_eq!(pending.started_at, Timestamp("query_start".to_owned()));
        assert!(state.pending_statement_prepare().is_none());
        assert!(events.events.is_empty());
    }

    #[test]
    fn mysql_adapter_emits_ok_sql_event_when_backend_ok_finalizes_pending_query() {
        let adapter =
            MysqlProtocolAdapter::with_clock(manual_clock(&[(0, "query_start"), (7, "query_end")]));
        let mut state = adapter.create_connection_state(&test_context());
        let mut events = VecCaptureEventEmitter::default();
        observe_authenticated_connection(&adapter, state.as_mut(), &mut events);

        adapter
            .observe_client_bytes(
                state.as_mut(),
                &com_query_packet("select 1", 0),
                &mut events,
            )
            .expect("COM_QUERY should start pending query");
        let response = query_ok_packet();
        let observation = adapter
            .observe_backend_bytes(state.as_mut(), &response, &mut events)
            .expect("backend OK should finalize pending query");
        let state = state
            .as_ref()
            .as_any()
            .downcast_ref::<MysqlConnectionState>()
            .expect("state should downcast");
        let event = events.events.first().expect("SQL event should be emitted");

        assert_eq!(observation, ProtocolObservation::new(response.len(), 1));
        assert_eq!(events.events.len(), 1);
        assert!(state.pending_query().is_none());
        assert_sql_event(event, CaptureStatus::Ok, "select 1", "query_end", 7);
        assert_eq!(event.id, SqlEventId("conn_1_query_1".to_owned()));
        assert_eq!(event.error, None);
        assert_eq!(
            event.result,
            Some(ResultSummary {
                affected_rows: Some(0),
                returned_rows: None,
            })
        );
        assert_eq!(event.metadata.fields.len(), 3);
        assert_eq!(event.metadata.fields[2].key, "ok_status_flags");
        assert_eq!(event.metadata.fields[2].value, MetadataValue::Unsigned(2));
    }

    #[test]
    fn mysql_adapter_emits_error_sql_event_when_backend_err_finalizes_pending_query() {
        let adapter = MysqlProtocolAdapter::with_clock(manual_clock(&[
            (10, "query_start"),
            (42, "query_error"),
        ]));
        let mut state = adapter.create_connection_state(&test_context());
        let mut events = VecCaptureEventEmitter::default();
        observe_authenticated_connection(&adapter, state.as_mut(), &mut events);

        adapter
            .observe_client_bytes(
                state.as_mut(),
                &com_query_packet("select bad", 0),
                &mut events,
            )
            .expect("COM_QUERY should start pending query");
        let response = query_err_packet();
        let observation = adapter
            .observe_backend_bytes(state.as_mut(), &response, &mut events)
            .expect("backend ERR should finalize pending query");
        let state = state
            .as_ref()
            .as_any()
            .downcast_ref::<MysqlConnectionState>()
            .expect("state should downcast");
        let event = events.events.first().expect("SQL event should be emitted");

        assert_eq!(observation, ProtocolObservation::new(response.len(), 1));
        assert_eq!(events.events.len(), 1);
        assert!(state.pending_query().is_none());
        assert_sql_event(event, CaptureStatus::Error, "select bad", "query_error", 32);
        assert_eq!(
            event.error,
            Some(ErrorSummary {
                code: Some("1096".to_owned()),
                sql_state: Some("HY000".to_owned()),
                message: "No tables used".to_owned(),
                metadata: Some(ProtocolMetadata {
                    protocol: ProtocolName("mysql".to_owned()),
                    fields: vec![MetadataField {
                        key: "mysql_error_code".to_owned(),
                        value: MetadataValue::Unsigned(1096),
                    }],
                }),
            })
        );
        assert_eq!(event.result, None);
        assert_eq!(event.metadata.fields.len(), 2);
    }

    #[test]
    fn mysql_adapter_does_not_emit_terminal_response_without_pending_query() {
        let adapter = MysqlProtocolAdapter::with_clock(manual_clock(&[]));
        let mut state = adapter.create_connection_state(&test_context());
        let mut events = VecCaptureEventEmitter::default();
        observe_authenticated_connection(&adapter, state.as_mut(), &mut events);
        let response = query_ok_packet();

        let observation = adapter
            .observe_backend_bytes(state.as_mut(), &response, &mut events)
            .expect("backend OK without pending query should be non-fatal");
        let state = state
            .as_ref()
            .as_any()
            .downcast_ref::<MysqlConnectionState>()
            .expect("state should downcast");

        assert_eq!(observation, ProtocolObservation::new(response.len(), 0));
        assert!(state.pending_query().is_none());
        assert!(events.events.is_empty());
    }

    #[test]
    fn mysql_adapter_keeps_pending_query_for_unsupported_backend_response() {
        let adapter = MysqlProtocolAdapter::with_clock(manual_clock(&[(0, "query_start")]));
        let mut state = adapter.create_connection_state(&test_context());
        let mut events = VecCaptureEventEmitter::default();
        observe_authenticated_connection(&adapter, state.as_mut(), &mut events);

        adapter
            .observe_client_bytes(
                state.as_mut(),
                &com_query_packet("select 1", 0),
                &mut events,
            )
            .expect("COM_QUERY should start pending query");
        let response = unsupported_backend_response_packet();
        let observation = adapter
            .observe_backend_bytes(state.as_mut(), &response, &mut events)
            .expect("unsupported backend response should be non-fatal");
        let state = state
            .as_ref()
            .as_any()
            .downcast_ref::<MysqlConnectionState>()
            .expect("state should downcast");

        assert_eq!(observation, ProtocolObservation::new(response.len(), 0));
        assert!(state.pending_query().is_some());
        assert!(events.events.is_empty());
    }

    #[test]
    fn mysql_adapter_keeps_malformed_ok_summary_non_fatal() {
        let adapter =
            MysqlProtocolAdapter::with_clock(manual_clock(&[(0, "query_start"), (7, "query_end")]));
        let mut state = adapter.create_connection_state(&test_context());
        let mut events = VecCaptureEventEmitter::default();
        observe_authenticated_connection(&adapter, state.as_mut(), &mut events);

        adapter
            .observe_client_bytes(
                state.as_mut(),
                &com_query_packet("select malformed ok", 0),
                &mut events,
            )
            .expect("COM_QUERY should start pending query");
        let response = malformed_query_ok_packet();
        let observation = adapter
            .observe_backend_bytes(state.as_mut(), &response, &mut events)
            .expect("malformed OK summary should remain non-fatal");
        let state = state
            .as_ref()
            .as_any()
            .downcast_ref::<MysqlConnectionState>()
            .expect("state should downcast");
        let event = events.events.first().expect("SQL event should be emitted");

        assert_eq!(observation, ProtocolObservation::new(response.len(), 1));
        assert_eq!(events.events.len(), 1);
        assert!(state.pending_query().is_none());
        assert_sql_event(
            event,
            CaptureStatus::Ok,
            "select malformed ok",
            "query_end",
            7,
        );
        assert_eq!(event.result, None);
        assert_eq!(event.metadata.fields.len(), 2);
    }

    #[test]
    fn mysql_adapter_keeps_malformed_err_summary_non_fatal() {
        let adapter = MysqlProtocolAdapter::with_clock(manual_clock(&[
            (10, "query_start"),
            (42, "query_error"),
        ]));
        let mut state = adapter.create_connection_state(&test_context());
        let mut events = VecCaptureEventEmitter::default();
        observe_authenticated_connection(&adapter, state.as_mut(), &mut events);

        adapter
            .observe_client_bytes(
                state.as_mut(),
                &com_query_packet("select malformed err", 0),
                &mut events,
            )
            .expect("COM_QUERY should start pending query");
        let response = malformed_query_err_packet();
        let observation = adapter
            .observe_backend_bytes(state.as_mut(), &response, &mut events)
            .expect("malformed ERR summary should remain non-fatal");
        let state = state
            .as_ref()
            .as_any()
            .downcast_ref::<MysqlConnectionState>()
            .expect("state should downcast");
        let event = events.events.first().expect("SQL event should be emitted");

        assert_eq!(observation, ProtocolObservation::new(response.len(), 1));
        assert_eq!(events.events.len(), 1);
        assert!(state.pending_query().is_none());
        assert_sql_event(
            event,
            CaptureStatus::Error,
            "select malformed err",
            "query_error",
            32,
        );
        assert_eq!(event.error, None);
        assert_eq!(event.result, None);
        assert_eq!(event.metadata.fields.len(), 2);
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

    fn query_ok_packet() -> Vec<u8> {
        packet_with_sequence_id(vec![0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00], 1)
    }

    fn malformed_query_ok_packet() -> Vec<u8> {
        packet_with_sequence_id(vec![0x00], 1)
    }

    fn query_err_packet() -> Vec<u8> {
        let mut payload = vec![0xff];
        payload.extend_from_slice(&1096u16.to_le_bytes());
        payload.push(b'#');
        payload.extend_from_slice(b"HY000");
        payload.extend_from_slice(b"No tables used");

        packet_with_sequence_id(payload, 1)
    }

    fn malformed_query_err_packet() -> Vec<u8> {
        packet_with_sequence_id(vec![0xff], 1)
    }

    fn unsupported_backend_response_packet() -> Vec<u8> {
        packet_with_sequence_id(vec![0x01], 1)
    }

    fn prepare_ok_packet(
        statement_id: u32,
        num_columns: u16,
        num_params: u16,
        warning_count: Option<u16>,
        sequence_id: u8,
    ) -> Vec<u8> {
        let mut payload = vec![0x00];
        payload.extend_from_slice(&statement_id.to_le_bytes());
        payload.extend_from_slice(&num_columns.to_le_bytes());
        payload.extend_from_slice(&num_params.to_le_bytes());
        payload.push(0x00);
        if let Some(warning_count) = warning_count {
            payload.extend_from_slice(&warning_count.to_le_bytes());
        }

        packet_with_sequence_id(payload, sequence_id)
    }

    fn prepare_err_packet(sequence_id: u8) -> Vec<u8> {
        let mut payload = vec![0xff];
        payload.extend_from_slice(&1064u16.to_le_bytes());
        payload.push(b'#');
        payload.extend_from_slice(b"42000");
        payload.extend_from_slice(b"You have an error");

        packet_with_sequence_id(payload, sequence_id)
    }

    fn malformed_prepare_ok_packet() -> Vec<u8> {
        packet_with_sequence_id(vec![0x00, 0x01], 1)
    }

    fn com_query_packet(sql: &str, sequence_id: u8) -> Vec<u8> {
        let mut payload = vec![MYSQL_COM_QUERY];
        payload.extend_from_slice(sql.as_bytes());

        packet_with_sequence_id(payload, sequence_id)
    }

    fn com_stmt_prepare_packet(sql: &str, sequence_id: u8) -> Vec<u8> {
        let mut payload = vec![MYSQL_COM_STMT_PREPARE];
        payload.extend_from_slice(sql.as_bytes());

        packet_with_sequence_id(payload, sequence_id)
    }

    fn unsupported_command_packet() -> Vec<u8> {
        packet_with_sequence_id(vec![0x01, b'x'], 0)
    }

    fn invalid_utf8_com_query_packet() -> Vec<u8> {
        packet_with_sequence_id(vec![MYSQL_COM_QUERY, 0xff], 0)
    }

    fn invalid_utf8_com_stmt_prepare_packet() -> Vec<u8> {
        packet_with_sequence_id(vec![MYSQL_COM_STMT_PREPARE, 0xff], 0)
    }

    fn assert_sql_event(
        event: &SqlEvent,
        expected_status: CaptureStatus,
        expected_sql: &str,
        expected_ended_at: &str,
        expected_duration_ms: u64,
    ) {
        assert_eq!(event.timestamp, Timestamp("query_start".to_owned()));
        assert_eq!(event.protocol, ProtocolName(MYSQL_PROTOCOL_NAME.to_owned()));
        assert_eq!(event.database_type, DatabaseType("mysql".to_owned()));
        assert_eq!(event.connection_id, ConnectionId("conn_1".to_owned()));
        assert_eq!(event.client_addr, "127.0.0.1:51000");
        assert_eq!(event.backend_addr, "127.0.0.1:3306");
        assert_eq!(event.kind, SqlEventKind::Query);
        assert_eq!(event.status, expected_status);
        assert_eq!(event.duration, DurationMillis(expected_duration_ms));
        assert_eq!(event.original_sql, expected_sql);
        assert_eq!(event.normalized_sql, None);
        assert_eq!(event.expanded_sql, None);
        assert_eq!(event.fingerprint, None);
        assert!(event.parameters.is_empty());
        assert_eq!(
            event.timings.started_at,
            Timestamp("query_start".to_owned())
        );
        assert_eq!(event.timings.duration, DurationMillis(expected_duration_ms));
        assert_eq!(
            event.timings.ended_at,
            Some(Timestamp(expected_ended_at.to_owned()))
        );
        assert_eq!(event.metadata.protocol, ProtocolName("mysql".to_owned()));
        assert!(event.metadata.fields.len() >= 2);
        assert_eq!(event.metadata.fields[0].key, "command");
        assert_eq!(
            event.metadata.fields[0].value,
            MetadataValue::String("COM_QUERY".to_owned())
        );
        assert_eq!(event.metadata.fields[1].key, "command_sequence_id");
        assert_eq!(event.metadata.fields[1].value, MetadataValue::Unsigned(0));
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

    #[derive(Debug)]
    struct ManualMysqlObservationClock {
        times: Mutex<VecDeque<MysqlObservationTime>>,
    }

    impl ManualMysqlObservationClock {
        fn new(entries: &[(u64, &str)]) -> Self {
            let base = Instant::now();
            let times = entries
                .iter()
                .map(|(offset_ms, timestamp)| MysqlObservationTime {
                    timestamp: Timestamp((*timestamp).to_owned()),
                    monotonic: base + Duration::from_millis(*offset_ms),
                })
                .collect();

            Self {
                times: Mutex::new(times),
            }
        }
    }

    impl MysqlObservationClock for ManualMysqlObservationClock {
        fn now(&self) -> MysqlObservationTime {
            self.times
                .lock()
                .expect("manual clock lock should not be poisoned")
                .pop_front()
                .expect("manual clock should have a queued observation time")
        }
    }

    fn manual_clock(entries: &[(u64, &str)]) -> Arc<ManualMysqlObservationClock> {
        Arc::new(ManualMysqlObservationClock::new(entries))
    }
}
