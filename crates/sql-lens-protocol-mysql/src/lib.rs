//! MySQL-compatible protocol adapter for SQL Lens.

mod authentication;
mod command;
mod err;
mod execute;
mod handshake;
mod ok;
mod packet;
mod prepare;

use std::{
    collections::BTreeMap,
    sync::Arc,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use sql_lens_core::{
    CaptureStatus, ConnectionInfo, ConnectionState, DurationMillis, ErrorSummary, MetadataField,
    MetadataValue, ProtocolMetadata, ProtocolName, QueryTiming, ResultSummary, SqlEvent,
    SqlEventId, SqlEventKind, SqlParameter, Timestamp, fingerprint_sql,
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
    MYSQL_CLIENT_QUERY_ATTRIBUTES, MYSQL_COM_PING, MYSQL_COM_QUERY, MYSQL_COM_QUIT,
    MYSQL_COM_STMT_CLOSE, MYSQL_COM_STMT_EXECUTE, MYSQL_COM_STMT_PREPARE, MysqlClientCommand,
    MysqlComPing, MysqlComQuery, MysqlComQuit, MysqlComStmtClose, MysqlComStmtExecute,
    MysqlComStmtPrepare, MysqlCommandKind, MysqlCommandParseError, MysqlParsedClientCommand,
    parse_client_command, parse_client_command_with_capabilities,
};
pub use err::{MysqlErrPacketParseError, MysqlErrPacketSummary, parse_err_packet_summary};
pub use execute::{
    MysqlDecodedParameter, MysqlDecodedParameters, MysqlExecuteParseError,
    MysqlExpandedSqlRenderError, MysqlNullBitmap, MysqlParameterType, decode_null_bitmap,
    decode_numeric_parameters, decode_parameters, render_expanded_sql,
};
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

const MYSQL_COM_STMT_EXECUTE_PARAMETER_PAYLOAD_OFFSET: usize = 10;

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
        } else if state.observe_backend_statement_execute_response(
            bytes,
            events,
            self.clock.as_ref(),
        ) {
            1
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
enum MysqlQueryResponseState {
    Columns { remaining_columns: u64 },
    AwaitingColumnTerminator,
    Rows { returned_rows: u64 },
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

#[derive(Debug, Clone, PartialEq)]
pub struct MysqlStatementExecuteEnvelope {
    pub command: MysqlClientCommand,
    pub started_at: Timestamp,
    pub started_monotonic: Instant,
    pub statement_id: u32,
    pub flags: u8,
    pub iteration_count: u32,
    pub has_parameter_payload: bool,
    pub statement: Option<MysqlPreparedStatement>,
    pub null_parameter_indexes: Vec<usize>,
    pub parameters: Vec<MysqlDecodedParameter>,
    pub expanded_sql: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MysqlPreparedStatement {
    pub statement_id: u32,
    pub template_sql: String,
    pub num_columns: u16,
    pub num_params: u16,
    pub warning_count: Option<u16>,
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
    pending_query_response: Option<MysqlQueryResponseState>,
    pending_statement_prepare: Option<MysqlPendingStatementPrepare>,
    last_statement_prepare_outcome: Option<MysqlStatementPrepareOutcome>,
    prepared_statements: BTreeMap<u32, MysqlPreparedStatement>,
    last_statement_execute_envelope: Option<MysqlStatementExecuteEnvelope>,
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
            pending_query_response: None,
            pending_statement_prepare: None,
            last_statement_prepare_outcome: None,
            prepared_statements: BTreeMap::new(),
            last_statement_execute_envelope: None,
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

    pub fn connection(&self) -> &ConnectionInfo {
        &self.connection
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

    pub fn prepared_statement(&self, statement_id: u32) -> Option<&MysqlPreparedStatement> {
        self.prepared_statements.get(&statement_id)
    }

    pub fn prepared_statement_count(&self) -> usize {
        self.prepared_statements.len()
    }

    pub fn last_statement_execute_envelope(&self) -> Option<&MysqlStatementExecuteEnvelope> {
        self.last_statement_execute_envelope.as_ref()
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

        let client_capability_flags = self
            .client_handshake
            .as_ref()
            .map(|handshake| handshake.capability_flags)
            .unwrap_or_default();
        let Ok(Some(command)) =
            parse_client_command_with_capabilities(packet.payload, client_capability_flags)
        else {
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
                self.pending_query_response = None;
            }
            MysqlParsedClientCommand::Ping(_) => {
                let time = clock.now();

                self.connection.last_activity_at = Some(time.timestamp);
                self.last_client_command = Some(MysqlClientCommand {
                    kind: MysqlCommandKind::Ping,
                    sequence_id: packet.header.sequence_id,
                    sql: String::new(),
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
            MysqlParsedClientCommand::StatementExecute(execute) => {
                let statement = self.prepared_statement(execute.statement_id).cloned();
                let (null_parameter_indexes, parameters, expanded_sql) =
                    if let Some(statement) = &statement {
                        let parameter_payload = packet
                            .payload
                            .get(MYSQL_COM_STMT_EXECUTE_PARAMETER_PAYLOAD_OFFSET..)
                            .unwrap_or_default();
                        let Ok(null_bitmap) =
                            decode_null_bitmap(parameter_payload, statement.num_params)
                        else {
                            return;
                        };
                        let null_bitmap_bytes_consumed = null_bitmap.bytes_consumed;
                        let null_parameter_indexes = null_bitmap.null_parameter_indexes;
                        let parameter_value_payload = parameter_payload
                            .get(null_bitmap_bytes_consumed..)
                            .unwrap_or_default();
                        let Ok(decoded_parameters) = decode_parameters(
                            parameter_value_payload,
                            statement.num_params,
                            &null_parameter_indexes,
                        ) else {
                            return;
                        };
                        match decoded_parameters {
                            Some(decoded_parameters) => {
                                let parameters = decoded_parameters.parameters;
                                let Ok(expanded_sql) =
                                    render_expanded_sql(&statement.template_sql, &parameters)
                                else {
                                    return;
                                };

                                (null_parameter_indexes, parameters, Some(expanded_sql))
                            }
                            None => (null_parameter_indexes, Vec::new(), None),
                        }
                    } else {
                        (Vec::new(), Vec::new(), None)
                    };
                let time = clock.now();
                let client_command = MysqlClientCommand {
                    kind: MysqlCommandKind::StatementExecute,
                    sequence_id: packet.header.sequence_id,
                    sql: statement
                        .as_ref()
                        .map(|statement| statement.template_sql.clone())
                        .unwrap_or_default(),
                };

                self.last_client_command = Some(client_command.clone());
                self.last_statement_execute_envelope = Some(MysqlStatementExecuteEnvelope {
                    command: client_command,
                    started_at: time.timestamp,
                    started_monotonic: time.monotonic,
                    statement_id: execute.statement_id,
                    flags: execute.flags,
                    iteration_count: execute.iteration_count,
                    has_parameter_payload: execute.has_parameter_payload,
                    statement,
                    null_parameter_indexes,
                    parameters,
                    expanded_sql,
                });
            }
            MysqlParsedClientCommand::StatementClose(close) => {
                self.prepared_statements.remove(&close.statement_id);
                self.last_client_command = Some(MysqlClientCommand {
                    kind: MysqlCommandKind::StatementClose,
                    sequence_id: packet.header.sequence_id,
                    sql: String::new(),
                });
            }
            MysqlParsedClientCommand::Quit(_) => {
                let time = clock.now();

                self.connection.last_activity_at = Some(time.timestamp);
                self.connection.state = ConnectionState::Closing;
                self.last_client_command = Some(MysqlClientCommand {
                    kind: MysqlCommandKind::Quit,
                    sequence_id: packet.header.sequence_id,
                    sql: String::new(),
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

        let mut offset = 0;
        let mut events_emitted = 0;

        while offset < bytes.len() && self.pending_query.is_some() {
            let Ok(packet) = parse_mysql_packet(&bytes[offset..]) else {
                break;
            };
            let packet_len = MYSQL_PACKET_HEADER_LEN + packet.payload.len();

            events_emitted +=
                self.observe_backend_query_response_packet(packet.payload, events, clock);
            offset += packet_len;
        }

        events_emitted
    }

    fn observe_backend_query_response_packet(
        &mut self,
        payload: &[u8],
        events: &mut dyn CaptureEventEmitter,
        clock: &dyn MysqlObservationClock,
    ) -> usize {
        if self.pending_query_response.is_some() {
            if payload.first() == Some(&0xff) {
                return self.emit_terminal_query_response(
                    payload,
                    events,
                    clock,
                    CaptureStatus::Error,
                    None,
                );
            }

            return self.observe_backend_query_result_set_response(payload, events, clock);
        }

        if let Some(status) = query_terminal_status(payload) {
            return self.emit_terminal_query_response(payload, events, clock, status, None);
        }

        self.observe_backend_query_result_set_response(payload, events, clock)
    }

    fn emit_terminal_query_response(
        &mut self,
        payload: &[u8],
        events: &mut dyn CaptureEventEmitter,
        clock: &dyn MysqlObservationClock,
        status: CaptureStatus,
        returned_rows: Option<u64>,
    ) -> usize {
        let ok_summary = if status == CaptureStatus::Ok {
            parse_ok_packet_summary(payload).ok().flatten()
        } else {
            None
        };
        let err_summary = if status == CaptureStatus::Error {
            parse_err_packet_summary(payload).ok().flatten()
        } else {
            None
        };

        let Some(pending) = self.pending_query.take() else {
            return 0;
        };
        self.pending_query_response = None;
        let ended = clock.now();
        let event = self.query_event(
            pending,
            ended,
            status,
            ok_summary,
            err_summary,
            returned_rows,
        );

        events.emit(event);

        1
    }

    fn observe_backend_query_result_set_response(
        &mut self,
        payload: &[u8],
        events: &mut dyn CaptureEventEmitter,
        clock: &dyn MysqlObservationClock,
    ) -> usize {
        match self.pending_query_response.take() {
            None => {
                let Some(column_count) = result_set_column_count(payload) else {
                    return 0;
                };

                self.pending_query_response = Some(MysqlQueryResponseState::Columns {
                    remaining_columns: column_count,
                });
                0
            }
            Some(MysqlQueryResponseState::Columns { remaining_columns }) => {
                if remaining_columns > 1 {
                    self.pending_query_response = Some(MysqlQueryResponseState::Columns {
                        remaining_columns: remaining_columns - 1,
                    });
                } else {
                    self.pending_query_response =
                        Some(MysqlQueryResponseState::AwaitingColumnTerminator);
                }
                0
            }
            Some(MysqlQueryResponseState::AwaitingColumnTerminator) => {
                if is_result_set_terminator(payload) {
                    self.pending_query_response =
                        Some(MysqlQueryResponseState::Rows { returned_rows: 0 });
                } else {
                    self.pending_query_response =
                        Some(MysqlQueryResponseState::Rows { returned_rows: 1 });
                }
                0
            }
            Some(MysqlQueryResponseState::Rows { returned_rows }) => {
                if is_result_set_terminator(payload) {
                    self.emit_terminal_query_response(
                        payload,
                        events,
                        clock,
                        CaptureStatus::Ok,
                        Some(returned_rows),
                    )
                } else {
                    self.pending_query_response = Some(MysqlQueryResponseState::Rows {
                        returned_rows: returned_rows.saturating_add(1),
                    });
                    0
                }
            }
        }
    }

    fn observe_backend_statement_execute_response(
        &mut self,
        bytes: &[u8],
        events: &mut dyn CaptureEventEmitter,
        clock: &dyn MysqlObservationClock,
    ) -> bool {
        if self.phase != MysqlConnectionPhase::Authenticated
            || self.last_statement_execute_envelope.is_none()
        {
            return false;
        }

        let Ok(packet) = parse_mysql_packet(bytes) else {
            return false;
        };

        let Some(status) = query_terminal_status(packet.payload) else {
            return false;
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

        let Some(envelope) = self.last_statement_execute_envelope.take() else {
            return false;
        };
        let ended = clock.now();
        let event = self.statement_execute_event(envelope, ended, status, ok_summary, err_summary);

        events.emit(event);

        true
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

        let command = pending.command;
        let response = match response {
            MysqlComStmtPrepareResponse::Ok(ok) => {
                self.prepared_statements.insert(
                    ok.statement_id,
                    MysqlPreparedStatement {
                        statement_id: ok.statement_id,
                        template_sql: command.sql.clone(),
                        num_columns: ok.num_columns,
                        num_params: ok.num_params,
                        warning_count: ok.warning_count,
                    },
                );
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
        };

        self.last_statement_prepare_outcome = Some(MysqlStatementPrepareOutcome {
            command,
            response_sequence_id: packet.header.sequence_id,
            response,
        });

        true
    }

    fn statement_execute_event(
        &mut self,
        envelope: MysqlStatementExecuteEnvelope,
        ended: MysqlObservationTime,
        status: CaptureStatus,
        ok_summary: Option<MysqlOkPacketSummary>,
        err_summary: Option<MysqlErrPacketSummary>,
    ) -> SqlEvent {
        let duration = duration_millis(envelope.started_monotonic, ended.monotonic);
        let event_id = SqlEventId(format!(
            "{}_statement_execute_{}",
            self.connection.id.0, self.next_query_sequence
        ));
        self.next_query_sequence += 1;
        let result = ok_summary.map(|summary| ResultSummary {
            affected_rows: Some(summary.affected_rows),
            returned_rows: None,
        });
        let error = err_summary.map(mysql_error_summary);
        let template_sql = envelope
            .statement
            .as_ref()
            .map(|statement| statement.template_sql.as_str());
        let parameters = envelope
            .parameters
            .into_iter()
            .map(|parameter| SqlParameter {
                index: parameter.index,
                name: statement_parameter_name(template_sql, parameter.index),
                value: parameter.value,
                redacted: false,
            })
            .collect();
        let mut metadata_fields = vec![
            MetadataField {
                key: "command".to_owned(),
                value: MetadataValue::String("COM_STMT_EXECUTE".to_owned()),
            },
            MetadataField {
                key: "command_sequence_id".to_owned(),
                value: MetadataValue::Unsigned(u64::from(envelope.command.sequence_id)),
            },
            MetadataField {
                key: "statement_id".to_owned(),
                value: MetadataValue::Unsigned(u64::from(envelope.statement_id)),
            },
            MetadataField {
                key: "flags".to_owned(),
                value: MetadataValue::Unsigned(u64::from(envelope.flags)),
            },
            MetadataField {
                key: "iteration_count".to_owned(),
                value: MetadataValue::Unsigned(u64::from(envelope.iteration_count)),
            },
        ];

        if let Some(status_flags) = ok_summary.and_then(|summary| summary.status_flags) {
            metadata_fields.push(MetadataField {
                key: "ok_status_flags".to_owned(),
                value: MetadataValue::Unsigned(u64::from(status_flags)),
            });
        }

        let fingerprint = Some(fingerprint_sql(
            envelope
                .expanded_sql
                .as_deref()
                .unwrap_or(&envelope.command.sql),
        ));

        SqlEvent {
            id: event_id,
            timestamp: envelope.started_at.clone(),
            target_name: self.connection.target_name.clone(),
            protocol: self.connection.protocol.clone(),
            database_type: self.connection.database_type.clone(),
            connection_id: self.connection.id.clone(),
            client_addr: self.connection.client_addr.clone(),
            backend_addr: self.connection.backend_addr.clone(),
            user: self.connection.user.clone(),
            database: self.connection.database.clone(),
            kind: SqlEventKind::StatementExecute,
            status,
            duration,
            original_sql: envelope.command.sql,
            normalized_sql: None,
            expanded_sql: envelope.expanded_sql,
            fingerprint,
            parameters,
            result,
            error,
            timings: QueryTiming {
                started_at: envelope.started_at,
                ended_at: Some(ended.timestamp),
                duration,
            },
            metadata: ProtocolMetadata {
                protocol: ProtocolName(MYSQL_PROTOCOL_NAME.to_owned()),
                fields: metadata_fields,
            },
        }
    }

    fn query_event(
        &mut self,
        pending: MysqlPendingQuery,
        ended: MysqlObservationTime,
        status: CaptureStatus,
        ok_summary: Option<MysqlOkPacketSummary>,
        err_summary: Option<MysqlErrPacketSummary>,
        returned_rows: Option<u64>,
    ) -> SqlEvent {
        let duration = duration_millis(pending.started_monotonic, ended.monotonic);
        let event_id = SqlEventId(format!(
            "{}_query_{}",
            self.connection.id.0, self.next_query_sequence
        ));
        self.next_query_sequence += 1;
        let command_sequence_id = pending.command.sequence_id;
        let original_sql = pending.command.sql;
        let result = if let Some(returned_rows) = returned_rows {
            Some(ResultSummary {
                affected_rows: None,
                returned_rows: Some(returned_rows),
            })
        } else {
            ok_summary.map(|summary| ResultSummary {
                affected_rows: Some(summary.affected_rows),
                returned_rows: None,
            })
        };
        let error = err_summary.map(mysql_error_summary);
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

        let fingerprint = Some(fingerprint_sql(&original_sql));

        SqlEvent {
            id: event_id,
            timestamp: pending.started_at.clone(),
            target_name: self.connection.target_name.clone(),
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
            fingerprint,
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

fn mysql_error_summary(summary: MysqlErrPacketSummary) -> ErrorSummary {
    ErrorSummary {
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
    }
}

fn statement_parameter_name(template_sql: Option<&str>, parameter_index: u16) -> Option<String> {
    let template_sql = template_sql?;
    let placeholder_index = usize::from(parameter_index);
    let placeholder_position = nth_placeholder_position(template_sql, placeholder_index)?;
    assignment_identifier_before_placeholder(&template_sql[..placeholder_position])
}

fn nth_placeholder_position(template_sql: &str, placeholder_index: usize) -> Option<usize> {
    template_sql
        .match_indices('?')
        .nth(placeholder_index)
        .map(|(position, _)| position)
}

fn assignment_identifier_before_placeholder(prefix: &str) -> Option<String> {
    let before_equals = prefix.trim_end().strip_suffix('=')?.trim_end();
    let identifier_end = before_equals.len();
    let identifier_start = before_equals[..identifier_end]
        .rfind(|value: char| !(value.is_ascii_alphanumeric() || value == '_'))
        .map_or(0, |position| position + 1);
    let identifier = before_equals[identifier_start..identifier_end].trim_matches('`');

    if identifier.is_empty() {
        None
    } else {
        Some(identifier.to_owned())
    }
}

fn query_terminal_status(payload: &[u8]) -> Option<CaptureStatus> {
    match payload.first() {
        Some(0x00) => Some(CaptureStatus::Ok),
        Some(0xff) => Some(CaptureStatus::Error),
        _ => None,
    }
}

fn result_set_column_count(payload: &[u8]) -> Option<u64> {
    let (column_count, _) = read_lenenc_integer(payload)?;
    (column_count > 0).then_some(column_count)
}

fn is_result_set_terminator(payload: &[u8]) -> bool {
    is_eof_packet(payload) || parse_ok_packet_summary(payload).ok().flatten().is_some()
}

fn is_eof_packet(payload: &[u8]) -> bool {
    payload.first() == Some(&0xfe) && payload.len() < 9
}

fn read_lenenc_integer(input: &[u8]) -> Option<(u64, usize)> {
    let (&first, rest) = input.split_first()?;

    match first {
        0x00..=0xfa => Some((u64::from(first), 1)),
        0xfc => read_fixed_lenenc_integer(rest, 2).map(|value| (value, 3)),
        0xfd => read_fixed_lenenc_integer(rest, 3).map(|value| (value, 4)),
        0xfe => read_fixed_lenenc_integer(rest, 8).map(|value| (value, 9)),
        _ => None,
    }
}

fn read_fixed_lenenc_integer(input: &[u8], len: usize) -> Option<u64> {
    if input.len() < len {
        return None;
    }

    let mut value = 0_u64;
    for (index, byte) in input[..len].iter().enumerate() {
        value |= u64::from(*byte) << (index * 8);
    }

    Some(value)
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
        SqlParameterValue, Timestamp,
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
        assert_eq!(state.prepared_statement_count(), 0);
        assert!(state.last_statement_execute_envelope().is_none());
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
        assert!(state.last_statement_execute_envelope().is_none());
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
        assert!(state.last_statement_execute_envelope().is_none());
        assert!(events.events.is_empty());
    }

    #[test]
    fn mysql_adapter_does_not_observe_com_stmt_execute_before_authentication() {
        let adapter = MysqlProtocolAdapter::new();
        let mut state = adapter.create_connection_state(&test_context());
        let mut events = VecCaptureEventEmitter::default();
        observe_complete_handshake(&adapter, state.as_mut(), &mut events);
        let packet = com_stmt_execute_packet(0x1122_3344, 0, 1, &[], 0);

        let observation = adapter
            .observe_client_bytes(state.as_mut(), &packet, &mut events)
            .expect("client execute bytes should be observed");
        let state = state
            .as_ref()
            .as_any()
            .downcast_ref::<MysqlConnectionState>()
            .expect("state should downcast");

        assert_eq!(observation, ProtocolObservation::new(packet.len(), 0));
        assert_eq!(state.phase(), MysqlConnectionPhase::ClientHandshakeSeen);
        assert!(state.last_client_command().is_none());
        assert!(state.last_statement_execute_envelope().is_none());
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
    fn mysql_adapter_observes_com_query_with_empty_query_attributes() {
        let adapter = MysqlProtocolAdapter::new();
        let mut state = adapter.create_connection_state(&test_context());
        let mut events = VecCaptureEventEmitter::default();
        observe_complete_handshake_with_client_packet(
            &adapter,
            state.as_mut(),
            &mut events,
            &client_handshake_response_with_query_attributes_packet(),
        );
        adapter
            .observe_backend_bytes(state.as_mut(), &authentication_ok_packet(), &mut events)
            .expect("authentication OK should be observed");
        let packet = attributed_com_query_packet("DO 1", 0);

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
        assert_eq!(command.kind, MysqlCommandKind::Query);
        assert_eq!(command.sql, "DO 1");
        assert!(state.pending_query().is_some());
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
    fn mysql_adapter_observes_com_stmt_execute_with_known_statement_id() {
        let adapter = MysqlProtocolAdapter::with_clock(manual_clock(&[(0, "execute_start")]));
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
        adapter
            .observe_backend_bytes(
                state.as_mut(),
                &prepare_ok_packet(0x1122_3344, 3, 2, Some(7), 1),
                &mut events,
            )
            .expect("prepare OK should be observed");
        let packet = com_stmt_execute_packet(0x1122_3344, 0, 1, &[0x01, 0x00], 2);

        let observation = adapter
            .observe_client_bytes(state.as_mut(), &packet, &mut events)
            .expect("COM_STMT_EXECUTE should be observed");
        let state = state
            .as_ref()
            .as_any()
            .downcast_ref::<MysqlConnectionState>()
            .expect("state should downcast");
        let command = state
            .last_client_command()
            .expect("client command should be stored");
        let envelope = state
            .last_statement_execute_envelope()
            .expect("execute envelope should be stored");

        assert_eq!(observation, ProtocolObservation::new(packet.len(), 0));
        assert_eq!(state.phase(), MysqlConnectionPhase::Authenticated);
        assert_eq!(command.kind, MysqlCommandKind::StatementExecute);
        assert_eq!(command.sequence_id, 2);
        assert_eq!(command.sql, "select * from users where id = ?");
        assert_eq!(&envelope.command, command);
        assert_eq!(envelope.statement_id, 0x1122_3344);
        assert_eq!(envelope.flags, 0);
        assert_eq!(envelope.iteration_count, 1);
        assert!(envelope.has_parameter_payload);
        assert_eq!(envelope.null_parameter_indexes, [0]);
        assert!(envelope.parameters.is_empty());
        assert_eq!(
            envelope.statement.as_ref(),
            Some(&MysqlPreparedStatement {
                statement_id: 0x1122_3344,
                template_sql: "select * from users where id = ?".to_owned(),
                num_columns: 3,
                num_params: 2,
                warning_count: Some(7),
            })
        );
        assert!(state.pending_query().is_none());
        assert!(events.events.is_empty());
    }

    #[test]
    fn mysql_adapter_observes_com_stmt_execute_numeric_parameters_with_known_statement_id() {
        let adapter = MysqlProtocolAdapter::with_clock(manual_clock(&[(0, "execute_start")]));
        let mut state = adapter.create_connection_state(&test_context());
        let mut events = VecCaptureEventEmitter::default();
        observe_authenticated_connection(&adapter, state.as_mut(), &mut events);

        adapter
            .observe_client_bytes(
                state.as_mut(),
                &com_stmt_prepare_packet("select numeric_params(?, ?, ?)", 0),
                &mut events,
            )
            .expect("COM_STMT_PREPARE should start pending prepare");
        adapter
            .observe_backend_bytes(
                state.as_mut(),
                &prepare_ok_packet(0x1122_3344, 0, 3, None, 1),
                &mut events,
            )
            .expect("prepare OK should be observed");
        let mut parameter_payload = vec![0b0000_0010, 0x01];
        parameter_payload.extend_from_slice(&[0x03, 0x00, 0x08, 0x80, 0x05, 0x00]);
        parameter_payload.extend_from_slice(&i32::to_le_bytes(-42));
        parameter_payload.extend_from_slice(&f64::to_le_bytes(2.5));
        let packet = com_stmt_execute_packet(0x1122_3344, 0, 1, &parameter_payload, 2);

        let observation = adapter
            .observe_client_bytes(state.as_mut(), &packet, &mut events)
            .expect("COM_STMT_EXECUTE should be observed");
        let state = state
            .as_ref()
            .as_any()
            .downcast_ref::<MysqlConnectionState>()
            .expect("state should downcast");
        let envelope = state
            .last_statement_execute_envelope()
            .expect("execute envelope should be stored");

        assert_eq!(observation, ProtocolObservation::new(packet.len(), 0));
        assert_eq!(envelope.null_parameter_indexes, [1]);
        assert_eq!(
            envelope.parameters,
            [
                MysqlDecodedParameter {
                    index: 0,
                    value: SqlParameterValue::Integer(-42),
                },
                MysqlDecodedParameter {
                    index: 1,
                    value: SqlParameterValue::Null,
                },
                MysqlDecodedParameter {
                    index: 2,
                    value: SqlParameterValue::Float(2.5),
                },
            ]
        );
        assert_eq!(
            envelope.expanded_sql.as_deref(),
            Some("select numeric_params(-42, NULL, 2.5)")
        );
        assert!(events.events.is_empty());
    }

    #[test]
    fn mysql_adapter_observes_com_stmt_execute_text_and_binary_parameters_with_known_statement_id()
    {
        const MYSQL_TYPE_BLOB: u8 = 0xfc;
        const MYSQL_TYPE_VAR_STRING: u8 = 0xfd;
        const MYSQL_TYPE_STRING: u8 = 0xfe;

        let adapter = MysqlProtocolAdapter::with_clock(manual_clock(&[(0, "execute_start")]));
        let mut state = adapter.create_connection_state(&test_context());
        let mut events = VecCaptureEventEmitter::default();
        observe_authenticated_connection(&adapter, state.as_mut(), &mut events);

        adapter
            .observe_client_bytes(
                state.as_mut(),
                &com_stmt_prepare_packet("select text_binary_params(?, ?, ?)", 0),
                &mut events,
            )
            .expect("COM_STMT_PREPARE should start pending prepare");
        adapter
            .observe_backend_bytes(
                state.as_mut(),
                &prepare_ok_packet(0x2233_4455, 0, 3, None, 1),
                &mut events,
            )
            .expect("prepare OK should be observed");
        let long_binary: Vec<u8> = (0..20).collect();
        let mut parameter_payload = vec![0x00, 0x01];
        parameter_payload.extend_from_slice(&[
            MYSQL_TYPE_VAR_STRING,
            0x00,
            MYSQL_TYPE_BLOB,
            0x00,
            MYSQL_TYPE_STRING,
            0x00,
        ]);
        parameter_payload.extend_from_slice(&length_encoded_value(b"alpha"));
        parameter_payload.extend_from_slice(&length_encoded_value(&long_binary));
        parameter_payload.extend_from_slice(&length_encoded_value(b"omega"));
        let packet = com_stmt_execute_packet(0x2233_4455, 0, 1, &parameter_payload, 2);

        let observation = adapter
            .observe_client_bytes(state.as_mut(), &packet, &mut events)
            .expect("COM_STMT_EXECUTE should be observed");
        let state = state
            .as_ref()
            .as_any()
            .downcast_ref::<MysqlConnectionState>()
            .expect("state should downcast");
        let envelope = state
            .last_statement_execute_envelope()
            .expect("execute envelope should be stored");

        assert_eq!(observation, ProtocolObservation::new(packet.len(), 0));
        assert!(envelope.null_parameter_indexes.is_empty());
        assert_eq!(
            envelope.parameters,
            [
                MysqlDecodedParameter {
                    index: 0,
                    value: SqlParameterValue::String("alpha".to_owned()),
                },
                MysqlDecodedParameter {
                    index: 1,
                    value: SqlParameterValue::BinarySummary(
                        "len=20 hex=000102030405060708090a0b0c0d0e0f...".to_owned()
                    ),
                },
                MysqlDecodedParameter {
                    index: 2,
                    value: SqlParameterValue::String("omega".to_owned()),
                },
            ]
        );
        assert_eq!(
            envelope.expanded_sql.as_deref(),
            Some(
                "select text_binary_params('alpha', 'len=20 hex=000102030405060708090a0b0c0d0e0f...', 'omega')"
            )
        );
        assert!(events.events.is_empty());
    }

    #[test]
    fn mysql_adapter_emits_statement_execute_event_when_backend_ok_finalizes_execute() {
        const MYSQL_TYPE_VAR_STRING: u8 = 0xfd;

        let adapter = MysqlProtocolAdapter::with_clock(manual_clock(&[
            (10, "execute_start"),
            (42, "execute_end"),
        ]));
        let mut state = adapter.create_connection_state(&test_context());
        let mut events = VecCaptureEventEmitter::default();
        observe_authenticated_connection(&adapter, state.as_mut(), &mut events);

        adapter
            .observe_client_bytes(
                state.as_mut(),
                &com_stmt_prepare_packet(
                    "update users set name = ?, password = ? where id = 42",
                    0,
                ),
                &mut events,
            )
            .expect("COM_STMT_PREPARE should start pending prepare");
        adapter
            .observe_backend_bytes(
                state.as_mut(),
                &prepare_ok_packet(0x5566_7788, 0, 2, None, 1),
                &mut events,
            )
            .expect("prepare OK should be observed");
        let mut parameter_payload = vec![0x00, 0x01];
        parameter_payload.extend_from_slice(&[
            MYSQL_TYPE_VAR_STRING,
            0x00,
            MYSQL_TYPE_VAR_STRING,
            0x00,
        ]);
        parameter_payload.extend_from_slice(&length_encoded_value(b"alice"));
        parameter_payload.extend_from_slice(&length_encoded_value(b"s3cr3t"));
        adapter
            .observe_client_bytes(
                state.as_mut(),
                &com_stmt_execute_packet(0x5566_7788, 0, 1, &parameter_payload, 2),
                &mut events,
            )
            .expect("COM_STMT_EXECUTE should be observed");

        let response = query_ok_packet();
        let observation = adapter
            .observe_backend_bytes(state.as_mut(), &response, &mut events)
            .expect("backend OK should finalize statement execute");
        let state = state
            .as_ref()
            .as_any()
            .downcast_ref::<MysqlConnectionState>()
            .expect("state should downcast");
        let event = events.events.first().expect("SQL event should be emitted");

        assert_eq!(observation, ProtocolObservation::new(response.len(), 1));
        assert_eq!(events.events.len(), 1);
        assert!(state.last_statement_execute_envelope().is_none());
        assert_eq!(
            event.id,
            SqlEventId("conn_1_statement_execute_1".to_owned())
        );
        assert_eq!(event.timestamp, Timestamp("execute_start".to_owned()));
        assert_eq!(event.target_name.as_deref(), Some("mysql-local"));
        assert_eq!(event.database_type, DatabaseType("mysql".to_owned()));
        assert_eq!(event.kind, SqlEventKind::StatementExecute);
        assert_eq!(event.status, CaptureStatus::Ok);
        assert_eq!(event.duration, DurationMillis(32));
        assert_eq!(
            event.original_sql,
            "update users set name = ?, password = ? where id = 42"
        );
        assert_eq!(
            event.expanded_sql.as_deref(),
            Some("update users set name = 'alice', password = 's3cr3t' where id = 42")
        );
        assert_eq!(
            event.fingerprint.as_deref(),
            Some("update users set name=?,password=? where id=?")
        );
        assert_eq!(event.parameters.len(), 2);
        assert_eq!(event.parameters[0].index, 0);
        assert_eq!(event.parameters[0].name.as_deref(), Some("name"));
        assert_eq!(
            event.parameters[0].value,
            SqlParameterValue::String("alice".to_owned())
        );
        assert_eq!(event.parameters[1].index, 1);
        assert_eq!(event.parameters[1].name.as_deref(), Some("password"));
        assert_eq!(
            event.parameters[1].value,
            SqlParameterValue::String("s3cr3t".to_owned())
        );
        assert_eq!(
            event.result,
            Some(ResultSummary {
                affected_rows: Some(0),
                returned_rows: None,
            })
        );
        assert_eq!(event.metadata.fields[0].key, "command");
        assert_eq!(
            event.metadata.fields[0].value,
            MetadataValue::String("COM_STMT_EXECUTE".to_owned())
        );
        assert_eq!(event.metadata.fields[2].key, "statement_id");
        assert_eq!(
            event.metadata.fields[2].value,
            MetadataValue::Unsigned(0x5566_7788)
        );
    }

    #[test]
    fn mysql_adapter_observes_com_stmt_execute_temporal_parameters_with_known_statement_id() {
        const MYSQL_TYPE_TIMESTAMP: u8 = 0x07;
        const MYSQL_TYPE_DATE: u8 = 0x0a;
        const MYSQL_TYPE_TIME: u8 = 0x0b;
        const MYSQL_TYPE_DATETIME: u8 = 0x0c;

        let adapter = MysqlProtocolAdapter::with_clock(manual_clock(&[(0, "execute_start")]));
        let mut state = adapter.create_connection_state(&test_context());
        let mut events = VecCaptureEventEmitter::default();
        observe_authenticated_connection(&adapter, state.as_mut(), &mut events);

        adapter
            .observe_client_bytes(
                state.as_mut(),
                &com_stmt_prepare_packet("select temporal_params(?, ?, ?, ?)", 0),
                &mut events,
            )
            .expect("COM_STMT_PREPARE should start pending prepare");
        adapter
            .observe_backend_bytes(
                state.as_mut(),
                &prepare_ok_packet(0x3344_5566, 0, 4, None, 1),
                &mut events,
            )
            .expect("prepare OK should be observed");
        let mut parameter_payload = vec![0x00, 0x01];
        parameter_payload.extend_from_slice(&[
            MYSQL_TYPE_DATE,
            0x00,
            MYSQL_TYPE_TIME,
            0x00,
            MYSQL_TYPE_DATETIME,
            0x00,
            MYSQL_TYPE_TIMESTAMP,
            0x00,
        ]);
        parameter_payload.extend_from_slice(&mysql_date_value(2026, 7, 7));
        parameter_payload.extend_from_slice(&mysql_time_value(true, 1, 2, 3, 4, Some(500)));
        parameter_payload.extend_from_slice(&mysql_datetime_value(2026, 7, 7, 9, 10, 11, None));
        parameter_payload.extend_from_slice(&mysql_datetime_value(
            2026,
            12,
            31,
            23,
            59,
            58,
            Some(123_456),
        ));
        let packet = com_stmt_execute_packet(0x3344_5566, 0, 1, &parameter_payload, 2);

        let observation = adapter
            .observe_client_bytes(state.as_mut(), &packet, &mut events)
            .expect("COM_STMT_EXECUTE should be observed");
        let state = state
            .as_ref()
            .as_any()
            .downcast_ref::<MysqlConnectionState>()
            .expect("state should downcast");
        let envelope = state
            .last_statement_execute_envelope()
            .expect("execute envelope should be stored");

        assert_eq!(observation, ProtocolObservation::new(packet.len(), 0));
        assert!(envelope.null_parameter_indexes.is_empty());
        assert_eq!(
            envelope.parameters,
            [
                MysqlDecodedParameter {
                    index: 0,
                    value: SqlParameterValue::Date("2026-07-07".to_owned()),
                },
                MysqlDecodedParameter {
                    index: 1,
                    value: SqlParameterValue::Time("-1 02:03:04.000500".to_owned()),
                },
                MysqlDecodedParameter {
                    index: 2,
                    value: SqlParameterValue::Timestamp("2026-07-07 09:10:11".to_owned()),
                },
                MysqlDecodedParameter {
                    index: 3,
                    value: SqlParameterValue::Timestamp("2026-12-31 23:59:58.123456".to_owned()),
                },
            ]
        );
        assert!(events.events.is_empty());
    }

    #[test]
    fn mysql_adapter_observes_com_stmt_execute_with_unknown_statement_id() {
        let adapter = MysqlProtocolAdapter::with_clock(manual_clock(&[(0, "execute_start")]));
        let mut state = adapter.create_connection_state(&test_context());
        let mut events = VecCaptureEventEmitter::default();
        observe_authenticated_connection(&adapter, state.as_mut(), &mut events);
        let packet = com_stmt_execute_packet(404, 2, 3, &[], 7);

        let observation = adapter
            .observe_client_bytes(state.as_mut(), &packet, &mut events)
            .expect("unknown statement execute should remain non-fatal");
        let state = state
            .as_ref()
            .as_any()
            .downcast_ref::<MysqlConnectionState>()
            .expect("state should downcast");
        let command = state
            .last_client_command()
            .expect("client command should be stored");
        let envelope = state
            .last_statement_execute_envelope()
            .expect("execute envelope should be stored");

        assert_eq!(observation, ProtocolObservation::new(packet.len(), 0));
        assert_eq!(state.phase(), MysqlConnectionPhase::Authenticated);
        assert_eq!(command.kind, MysqlCommandKind::StatementExecute);
        assert_eq!(command.sequence_id, 7);
        assert_eq!(command.sql, "");
        assert_eq!(&envelope.command, command);
        assert_eq!(envelope.statement_id, 404);
        assert_eq!(envelope.flags, 2);
        assert_eq!(envelope.iteration_count, 3);
        assert!(!envelope.has_parameter_payload);
        assert!(envelope.statement.is_none());
        assert!(envelope.null_parameter_indexes.is_empty());
        assert!(envelope.parameters.is_empty());
        assert!(envelope.expanded_sql.is_none());
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
        assert_eq!(state.prepared_statement_count(), 1);
        assert_eq!(
            state.prepared_statement(0x1122_3344),
            Some(&MysqlPreparedStatement {
                statement_id: 0x1122_3344,
                template_sql: "select * from users where id = ?".to_owned(),
                num_columns: 3,
                num_params: 2,
                warning_count: Some(7),
            })
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
        assert_eq!(state.prepared_statement_count(), 0);
        assert!(state.prepared_statement(0x1122_3344).is_none());
        assert!(events.events.is_empty());
    }

    #[test]
    fn mysql_adapter_replaces_prepared_statement_mapping_for_same_statement_id() {
        let adapter = MysqlProtocolAdapter::with_clock(manual_clock(&[]));
        let mut state = adapter.create_connection_state(&test_context());
        let mut events = VecCaptureEventEmitter::default();
        observe_authenticated_connection(&adapter, state.as_mut(), &mut events);

        adapter
            .observe_client_bytes(
                state.as_mut(),
                &com_stmt_prepare_packet("select old_template(?)", 0),
                &mut events,
            )
            .expect("first COM_STMT_PREPARE should start pending prepare");
        adapter
            .observe_backend_bytes(
                state.as_mut(),
                &prepare_ok_packet(42, 0, 1, Some(1), 1),
                &mut events,
            )
            .expect("first prepare OK should be observed");

        adapter
            .observe_client_bytes(
                state.as_mut(),
                &com_stmt_prepare_packet("select new_template(?, ?)", 0),
                &mut events,
            )
            .expect("second COM_STMT_PREPARE should start pending prepare");
        let response = prepare_ok_packet(42, 2, 2, None, 1);
        let observation = adapter
            .observe_backend_bytes(state.as_mut(), &response, &mut events)
            .expect("second prepare OK should be observed");
        let state = state
            .as_ref()
            .as_any()
            .downcast_ref::<MysqlConnectionState>()
            .expect("state should downcast");

        assert_eq!(observation, ProtocolObservation::new(response.len(), 0));
        assert_eq!(state.prepared_statement_count(), 1);
        assert_eq!(
            state.prepared_statement(42),
            Some(&MysqlPreparedStatement {
                statement_id: 42,
                template_sql: "select new_template(?, ?)".to_owned(),
                num_columns: 2,
                num_params: 2,
                warning_count: None,
            })
        );
        assert!(events.events.is_empty());
    }

    #[test]
    fn mysql_adapter_keeps_prepared_statement_mappings_connection_local() {
        let adapter = MysqlProtocolAdapter::with_clock(manual_clock(&[]));
        let mut first_state = adapter.create_connection_state(&test_context());
        let mut second_state = adapter.create_connection_state(&test_context());
        let mut events = VecCaptureEventEmitter::default();
        observe_authenticated_connection(&adapter, first_state.as_mut(), &mut events);
        observe_authenticated_connection(&adapter, second_state.as_mut(), &mut events);

        adapter
            .observe_client_bytes(
                first_state.as_mut(),
                &com_stmt_prepare_packet("select first_connection(?)", 0),
                &mut events,
            )
            .expect("COM_STMT_PREPARE should start pending prepare");
        adapter
            .observe_backend_bytes(
                first_state.as_mut(),
                &prepare_ok_packet(99, 0, 1, None, 1),
                &mut events,
            )
            .expect("prepare OK should be observed");
        let first_state = first_state
            .as_ref()
            .as_any()
            .downcast_ref::<MysqlConnectionState>()
            .expect("first state should downcast");
        let second_state = second_state
            .as_ref()
            .as_any()
            .downcast_ref::<MysqlConnectionState>()
            .expect("second state should downcast");

        assert_eq!(first_state.prepared_statement_count(), 1);
        assert!(first_state.prepared_statement(99).is_some());
        assert_eq!(second_state.prepared_statement_count(), 0);
        assert!(second_state.prepared_statement(99).is_none());
        assert!(events.events.is_empty());
    }

    #[test]
    fn mysql_adapter_removes_prepared_statement_on_com_stmt_close() {
        let adapter = MysqlProtocolAdapter::with_clock(manual_clock(&[]));
        let mut state = adapter.create_connection_state(&test_context());
        let mut events = VecCaptureEventEmitter::default();
        observe_authenticated_connection(&adapter, state.as_mut(), &mut events);

        adapter
            .observe_client_bytes(
                state.as_mut(),
                &com_stmt_prepare_packet("select close_me(?)", 0),
                &mut events,
            )
            .expect("COM_STMT_PREPARE should start pending prepare");
        adapter
            .observe_backend_bytes(
                state.as_mut(),
                &prepare_ok_packet(0x1122_3344, 0, 1, None, 1),
                &mut events,
            )
            .expect("prepare OK should be observed");
        let packet = com_stmt_close_packet(0x1122_3344, 3);

        let observation = adapter
            .observe_client_bytes(state.as_mut(), &packet, &mut events)
            .expect("COM_STMT_CLOSE should be observed");
        let state = state
            .as_ref()
            .as_any()
            .downcast_ref::<MysqlConnectionState>()
            .expect("state should downcast");
        let command = state
            .last_client_command()
            .expect("close command should be stored");

        assert_eq!(observation, ProtocolObservation::new(packet.len(), 0));
        assert_eq!(state.phase(), MysqlConnectionPhase::Authenticated);
        assert_eq!(state.prepared_statement_count(), 0);
        assert!(state.prepared_statement(0x1122_3344).is_none());
        assert_eq!(command.kind, MysqlCommandKind::StatementClose);
        assert_eq!(command.sequence_id, 3);
        assert_eq!(command.sql, "");
        assert!(state.pending_query().is_none());
        assert!(state.pending_statement_prepare().is_none());
        assert!(state.last_statement_execute_envelope().is_none());
        assert!(events.events.is_empty());
    }

    #[test]
    fn mysql_adapter_keeps_existing_statements_when_closing_unknown_statement() {
        let adapter = MysqlProtocolAdapter::with_clock(manual_clock(&[]));
        let mut state = adapter.create_connection_state(&test_context());
        let mut events = VecCaptureEventEmitter::default();
        observe_authenticated_connection(&adapter, state.as_mut(), &mut events);

        adapter
            .observe_client_bytes(
                state.as_mut(),
                &com_stmt_prepare_packet("select keep_me(?)", 0),
                &mut events,
            )
            .expect("COM_STMT_PREPARE should start pending prepare");
        adapter
            .observe_backend_bytes(
                state.as_mut(),
                &prepare_ok_packet(0x1122_3344, 0, 1, None, 1),
                &mut events,
            )
            .expect("prepare OK should be observed");
        let packet = com_stmt_close_packet(404, 3);

        let observation = adapter
            .observe_client_bytes(state.as_mut(), &packet, &mut events)
            .expect("unknown COM_STMT_CLOSE should remain harmless");
        let state = state
            .as_ref()
            .as_any()
            .downcast_ref::<MysqlConnectionState>()
            .expect("state should downcast");
        let command = state
            .last_client_command()
            .expect("close command should be stored");

        assert_eq!(observation, ProtocolObservation::new(packet.len(), 0));
        assert_eq!(state.prepared_statement_count(), 1);
        assert!(state.prepared_statement(0x1122_3344).is_some());
        assert_eq!(command.kind, MysqlCommandKind::StatementClose);
        assert_eq!(command.sequence_id, 3);
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
        assert_eq!(state.prepared_statement_count(), 0);
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
        assert!(state.last_statement_execute_envelope().is_none());
        assert!(events.events.is_empty());
    }

    #[test]
    fn mysql_adapter_keeps_authenticated_phase_for_malformed_com_stmt_execute() {
        let adapter = MysqlProtocolAdapter::new();
        let mut state = adapter.create_connection_state(&test_context());
        let mut events = VecCaptureEventEmitter::default();
        observe_authenticated_connection(&adapter, state.as_mut(), &mut events);
        let packet = malformed_com_stmt_execute_packet();

        let observation = adapter
            .observe_client_bytes(state.as_mut(), &packet, &mut events)
            .expect("malformed execute command should remain non-fatal");
        let state = state
            .as_ref()
            .as_any()
            .downcast_ref::<MysqlConnectionState>()
            .expect("state should downcast");

        assert_eq!(observation, ProtocolObservation::new(packet.len(), 0));
        assert_eq!(state.phase(), MysqlConnectionPhase::Authenticated);
        assert!(state.last_client_command().is_none());
        assert!(state.last_statement_execute_envelope().is_none());
        assert!(events.events.is_empty());
    }

    #[test]
    fn mysql_adapter_keeps_authenticated_phase_for_malformed_com_stmt_close() {
        let adapter = MysqlProtocolAdapter::with_clock(manual_clock(&[]));
        let mut state = adapter.create_connection_state(&test_context());
        let mut events = VecCaptureEventEmitter::default();
        observe_authenticated_connection(&adapter, state.as_mut(), &mut events);

        adapter
            .observe_client_bytes(
                state.as_mut(),
                &com_stmt_prepare_packet("select keep_after_malformed_close(?)", 0),
                &mut events,
            )
            .expect("COM_STMT_PREPARE should start pending prepare");
        adapter
            .observe_backend_bytes(
                state.as_mut(),
                &prepare_ok_packet(0x1122_3344, 0, 1, None, 1),
                &mut events,
            )
            .expect("prepare OK should be observed");
        let packet = malformed_com_stmt_close_packet();

        let observation = adapter
            .observe_client_bytes(state.as_mut(), &packet, &mut events)
            .expect("malformed close command should remain non-fatal");
        let state = state
            .as_ref()
            .as_any()
            .downcast_ref::<MysqlConnectionState>()
            .expect("state should downcast");
        let command = state
            .last_client_command()
            .expect("prepare command should remain the last stored command");

        assert_eq!(observation, ProtocolObservation::new(packet.len(), 0));
        assert_eq!(state.phase(), MysqlConnectionPhase::Authenticated);
        assert_eq!(state.prepared_statement_count(), 1);
        assert!(state.prepared_statement(0x1122_3344).is_some());
        assert_eq!(command.kind, MysqlCommandKind::StatementPrepare);
        assert!(events.events.is_empty());
    }

    #[test]
    fn mysql_adapter_keeps_authenticated_phase_for_truncated_execute_null_bitmap() {
        let adapter = MysqlProtocolAdapter::with_clock(manual_clock(&[]));
        let mut state = adapter.create_connection_state(&test_context());
        let mut events = VecCaptureEventEmitter::default();
        observe_authenticated_connection(&adapter, state.as_mut(), &mut events);

        adapter
            .observe_client_bytes(
                state.as_mut(),
                &com_stmt_prepare_packet("select many_params(?, ?, ?, ?, ?, ?, ?, ?, ?)", 0),
                &mut events,
            )
            .expect("COM_STMT_PREPARE should start pending prepare");
        adapter
            .observe_backend_bytes(
                state.as_mut(),
                &prepare_ok_packet(0x1122_3344, 0, 9, None, 1),
                &mut events,
            )
            .expect("prepare OK should be observed");
        let packet = com_stmt_execute_packet(0x1122_3344, 0, 1, &[0x00], 2);

        let observation = adapter
            .observe_client_bytes(state.as_mut(), &packet, &mut events)
            .expect("truncated NULL bitmap should remain non-fatal");
        let state = state
            .as_ref()
            .as_any()
            .downcast_ref::<MysqlConnectionState>()
            .expect("state should downcast");
        let command = state
            .last_client_command()
            .expect("prepare command should remain the last stored command");

        assert_eq!(observation, ProtocolObservation::new(packet.len(), 0));
        assert_eq!(state.phase(), MysqlConnectionPhase::Authenticated);
        assert_eq!(command.kind, MysqlCommandKind::StatementPrepare);
        assert!(state.last_statement_execute_envelope().is_none());
        assert!(events.events.is_empty());
    }

    #[test]
    fn mysql_adapter_keeps_authenticated_phase_for_truncated_execute_numeric_value() {
        let adapter = MysqlProtocolAdapter::with_clock(manual_clock(&[]));
        let mut state = adapter.create_connection_state(&test_context());
        let mut events = VecCaptureEventEmitter::default();
        observe_authenticated_connection(&adapter, state.as_mut(), &mut events);

        adapter
            .observe_client_bytes(
                state.as_mut(),
                &com_stmt_prepare_packet("select numeric_param(?)", 0),
                &mut events,
            )
            .expect("COM_STMT_PREPARE should start pending prepare");
        adapter
            .observe_backend_bytes(
                state.as_mut(),
                &prepare_ok_packet(0x1122_3344, 0, 1, None, 1),
                &mut events,
            )
            .expect("prepare OK should be observed");
        let packet = com_stmt_execute_packet(0x1122_3344, 0, 1, &[0x00, 0x01, 0x08, 0x00, 0x01], 2);

        let observation = adapter
            .observe_client_bytes(state.as_mut(), &packet, &mut events)
            .expect("truncated numeric value should remain non-fatal");
        let state = state
            .as_ref()
            .as_any()
            .downcast_ref::<MysqlConnectionState>()
            .expect("state should downcast");
        let command = state
            .last_client_command()
            .expect("prepare command should remain the last stored command");

        assert_eq!(observation, ProtocolObservation::new(packet.len(), 0));
        assert_eq!(state.phase(), MysqlConnectionPhase::Authenticated);
        assert_eq!(command.kind, MysqlCommandKind::StatementPrepare);
        assert!(state.last_statement_execute_envelope().is_none());
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
    fn mysql_adapter_updates_activity_for_com_ping_without_sql_event() {
        let adapter = MysqlProtocolAdapter::with_clock(manual_clock(&[(0, "ping_at")]));
        let mut state = adapter.create_connection_state(&test_context());
        let mut events = VecCaptureEventEmitter::default();
        observe_authenticated_connection(&adapter, state.as_mut(), &mut events);
        let packet = com_ping_packet(4);

        let observation = adapter
            .observe_client_bytes(state.as_mut(), &packet, &mut events)
            .expect("COM_PING should be observed");
        let state = state
            .as_ref()
            .as_any()
            .downcast_ref::<MysqlConnectionState>()
            .expect("state should downcast");
        let command = state
            .last_client_command()
            .expect("ping command should be stored");

        assert_eq!(observation, ProtocolObservation::new(packet.len(), 0));
        assert_eq!(state.phase(), MysqlConnectionPhase::Authenticated);
        assert_eq!(
            state.connection().last_activity_at,
            Some(Timestamp("ping_at".to_owned()))
        );
        assert_eq!(state.connection().state, ConnectionState::BackendConnected);
        assert_eq!(command.kind, MysqlCommandKind::Ping);
        assert_eq!(command.sequence_id, 4);
        assert_eq!(command.sql, "");
        assert!(state.pending_query().is_none());
        assert!(state.pending_statement_prepare().is_none());
        assert!(state.last_statement_execute_envelope().is_none());
        assert!(events.events.is_empty());
    }

    #[test]
    fn mysql_adapter_marks_connection_closing_for_com_quit_without_sql_event() {
        let adapter = MysqlProtocolAdapter::with_clock(manual_clock(&[(0, "quit_at")]));
        let mut state = adapter.create_connection_state(&test_context());
        let mut events = VecCaptureEventEmitter::default();
        observe_authenticated_connection(&adapter, state.as_mut(), &mut events);
        let packet = com_quit_packet(7);

        let observation = adapter
            .observe_client_bytes(state.as_mut(), &packet, &mut events)
            .expect("COM_QUIT should be observed");
        let state = state
            .as_ref()
            .as_any()
            .downcast_ref::<MysqlConnectionState>()
            .expect("state should downcast");
        let command = state
            .last_client_command()
            .expect("quit command should be stored");

        assert_eq!(observation, ProtocolObservation::new(packet.len(), 0));
        assert_eq!(state.phase(), MysqlConnectionPhase::Authenticated);
        assert_eq!(
            state.connection().last_activity_at,
            Some(Timestamp("quit_at".to_owned()))
        );
        assert_eq!(state.connection().state, ConnectionState::Closing);
        assert_eq!(command.kind, MysqlCommandKind::Quit);
        assert_eq!(command.sequence_id, 7);
        assert_eq!(command.sql, "");
        assert!(state.pending_query().is_none());
        assert!(state.pending_statement_prepare().is_none());
        assert!(state.last_statement_execute_envelope().is_none());
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
    fn mysql_adapter_emits_ok_sql_event_when_result_set_finalizes_pending_query() {
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
        let packets = [
            result_set_column_count_packet(1, 1),
            result_set_column_definition_packet(2),
            result_set_eof_packet(3),
            result_set_row_packet(&[b"1"], 4),
        ];
        for packet in packets {
            let observation = adapter
                .observe_backend_bytes(state.as_mut(), &packet, &mut events)
                .expect("result-set packet should be observed");

            assert_eq!(observation, ProtocolObservation::new(packet.len(), 0));
            assert!(events.events.is_empty());
        }

        let response = result_set_eof_packet(5);
        let observation = adapter
            .observe_backend_bytes(state.as_mut(), &response, &mut events)
            .expect("row terminator should finalize pending query");
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
        assert_eq!(
            event.result,
            Some(ResultSummary {
                affected_rows: None,
                returned_rows: Some(1),
            })
        );
    }

    #[test]
    fn mysql_adapter_counts_multiple_result_set_rows() {
        let adapter =
            MysqlProtocolAdapter::with_clock(manual_clock(&[(0, "query_start"), (7, "query_end")]));
        let mut state = adapter.create_connection_state(&test_context());
        let mut events = VecCaptureEventEmitter::default();
        observe_authenticated_connection(&adapter, state.as_mut(), &mut events);

        adapter
            .observe_client_bytes(
                state.as_mut(),
                &com_query_packet("select id from users", 0),
                &mut events,
            )
            .expect("COM_QUERY should start pending query");
        for packet in [
            result_set_column_count_packet(1, 1),
            result_set_column_definition_packet(2),
            result_set_eof_packet(3),
            result_set_row_packet(&[b"1"], 4),
            result_set_row_packet(&[b"2"], 5),
        ] {
            adapter
                .observe_backend_bytes(state.as_mut(), &packet, &mut events)
                .expect("result-set packet should be observed");
        }

        adapter
            .observe_backend_bytes(state.as_mut(), &result_set_eof_packet(6), &mut events)
            .expect("row terminator should finalize pending query");
        let event = events.events.first().expect("SQL event should be emitted");

        assert_eq!(
            event.result,
            Some(ResultSummary {
                affected_rows: None,
                returned_rows: Some(2),
            })
        );
    }

    #[test]
    fn mysql_adapter_handles_result_set_packets_in_one_backend_read() {
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
        let response = combined_packets([
            result_set_column_count_packet(1, 1),
            result_set_column_definition_packet(2),
            result_set_eof_packet(3),
            result_set_row_packet(&[b"1"], 4),
            result_set_eof_packet(5),
        ]);
        let observation = adapter
            .observe_backend_bytes(state.as_mut(), &response, &mut events)
            .expect("combined result-set packets should be observed");
        let event = events.events.first().expect("SQL event should be emitted");

        assert_eq!(observation, ProtocolObservation::new(response.len(), 1));
        assert_eq!(events.events.len(), 1);
        assert_sql_event(event, CaptureStatus::Ok, "select 1", "query_end", 7);
        assert_eq!(
            event.result,
            Some(ResultSummary {
                affected_rows: None,
                returned_rows: Some(1),
            })
        );
    }

    #[test]
    fn mysql_adapter_handles_result_set_without_column_terminator() {
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
        let response = combined_packets([
            result_set_column_count_packet(1, 1),
            result_set_column_definition_packet(2),
            result_set_row_packet(&[b"1"], 3),
            result_set_eof_packet(4),
        ]);
        let observation = adapter
            .observe_backend_bytes(state.as_mut(), &response, &mut events)
            .expect("combined result-set packets should be observed");
        let event = events.events.first().expect("SQL event should be emitted");

        assert_eq!(observation, ProtocolObservation::new(response.len(), 1));
        assert_eq!(
            event.result,
            Some(ResultSummary {
                affected_rows: None,
                returned_rows: Some(1),
            })
        );
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
            target_name: Some("mysql-local".to_owned()),
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

    fn client_handshake_response_with_query_attributes_packet() -> Vec<u8> {
        let mut payload = client_handshake_response_payload();
        let mut capability_flags =
            u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
        capability_flags |= MYSQL_CLIENT_QUERY_ATTRIBUTES;
        payload[..4].copy_from_slice(&capability_flags.to_le_bytes());

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

    fn result_set_column_count_packet(column_count: u8, sequence_id: u8) -> Vec<u8> {
        packet_with_sequence_id(vec![column_count], sequence_id)
    }

    fn result_set_column_definition_packet(sequence_id: u8) -> Vec<u8> {
        packet_with_sequence_id(b"def".to_vec(), sequence_id)
    }

    fn result_set_row_packet(values: &[&[u8]], sequence_id: u8) -> Vec<u8> {
        let mut payload = Vec::new();
        for value in values {
            payload.extend_from_slice(&length_encoded_value(value));
        }

        packet_with_sequence_id(payload, sequence_id)
    }

    fn result_set_eof_packet(sequence_id: u8) -> Vec<u8> {
        packet_with_sequence_id(vec![0xfe, 0x00, 0x00, 0x02, 0x00], sequence_id)
    }

    fn combined_packets<const N: usize>(packets: [Vec<u8>; N]) -> Vec<u8> {
        packets.into_iter().flatten().collect()
    }

    fn com_quit_packet(sequence_id: u8) -> Vec<u8> {
        packet_with_sequence_id(vec![MYSQL_COM_QUIT], sequence_id)
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

    fn attributed_com_query_packet(sql: &str, sequence_id: u8) -> Vec<u8> {
        let mut payload = vec![MYSQL_COM_QUERY, 0x00, 0x01];
        payload.extend_from_slice(sql.as_bytes());

        packet_with_sequence_id(payload, sequence_id)
    }

    fn com_ping_packet(sequence_id: u8) -> Vec<u8> {
        packet_with_sequence_id(vec![MYSQL_COM_PING], sequence_id)
    }

    fn com_stmt_prepare_packet(sql: &str, sequence_id: u8) -> Vec<u8> {
        let mut payload = vec![MYSQL_COM_STMT_PREPARE];
        payload.extend_from_slice(sql.as_bytes());

        packet_with_sequence_id(payload, sequence_id)
    }

    fn com_stmt_execute_packet(
        statement_id: u32,
        flags: u8,
        iteration_count: u32,
        parameter_payload: &[u8],
        sequence_id: u8,
    ) -> Vec<u8> {
        let mut payload = vec![MYSQL_COM_STMT_EXECUTE];
        payload.extend_from_slice(&statement_id.to_le_bytes());
        payload.push(flags);
        payload.extend_from_slice(&iteration_count.to_le_bytes());
        payload.extend_from_slice(parameter_payload);

        packet_with_sequence_id(payload, sequence_id)
    }

    fn com_stmt_close_packet(statement_id: u32, sequence_id: u8) -> Vec<u8> {
        let mut payload = vec![MYSQL_COM_STMT_CLOSE];
        payload.extend_from_slice(&statement_id.to_le_bytes());

        packet_with_sequence_id(payload, sequence_id)
    }

    fn length_encoded_value(bytes: &[u8]) -> Vec<u8> {
        let length = u8::try_from(bytes.len()).expect("test value should use one-byte length");
        let mut encoded = vec![length];
        encoded.extend_from_slice(bytes);

        encoded
    }

    fn mysql_date_value(year: u16, month: u8, day: u8) -> Vec<u8> {
        let mut value = Vec::new();
        value.extend_from_slice(&year.to_le_bytes());
        value.push(month);
        value.push(day);

        length_encoded_value(&value)
    }

    fn mysql_datetime_value(
        year: u16,
        month: u8,
        day: u8,
        hour: u8,
        minute: u8,
        second: u8,
        micros: Option<u32>,
    ) -> Vec<u8> {
        let mut value = Vec::new();
        value.extend_from_slice(&year.to_le_bytes());
        value.push(month);
        value.push(day);
        value.push(hour);
        value.push(minute);
        value.push(second);
        if let Some(micros) = micros {
            value.extend_from_slice(&micros.to_le_bytes());
        }

        length_encoded_value(&value)
    }

    fn mysql_time_value(
        is_negative: bool,
        days: u32,
        hour: u8,
        minute: u8,
        second: u8,
        micros: Option<u32>,
    ) -> Vec<u8> {
        let mut value = Vec::new();
        value.push(u8::from(is_negative));
        value.extend_from_slice(&days.to_le_bytes());
        value.push(hour);
        value.push(minute);
        value.push(second);
        if let Some(micros) = micros {
            value.extend_from_slice(&micros.to_le_bytes());
        }

        length_encoded_value(&value)
    }

    fn unsupported_command_packet() -> Vec<u8> {
        packet_with_sequence_id(vec![0x7f, b'x'], 0)
    }

    fn invalid_utf8_com_query_packet() -> Vec<u8> {
        packet_with_sequence_id(vec![MYSQL_COM_QUERY, 0xff], 0)
    }

    fn invalid_utf8_com_stmt_prepare_packet() -> Vec<u8> {
        packet_with_sequence_id(vec![MYSQL_COM_STMT_PREPARE, 0xff], 0)
    }

    fn malformed_com_stmt_execute_packet() -> Vec<u8> {
        packet_with_sequence_id(vec![MYSQL_COM_STMT_EXECUTE, 0x01], 0)
    }

    fn malformed_com_stmt_close_packet() -> Vec<u8> {
        packet_with_sequence_id(vec![MYSQL_COM_STMT_CLOSE, 0x01], 0)
    }

    fn assert_sql_event(
        event: &SqlEvent,
        expected_status: CaptureStatus,
        expected_sql: &str,
        expected_ended_at: &str,
        expected_duration_ms: u64,
    ) {
        assert_eq!(event.timestamp, Timestamp("query_start".to_owned()));
        assert_eq!(event.target_name.as_deref(), Some("mysql-local"));
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
        let expected_fingerprint = fingerprint_sql(expected_sql);
        assert_eq!(
            event.fingerprint.as_deref(),
            Some(expected_fingerprint.as_str())
        );
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
        observe_complete_handshake_with_client_packet(
            adapter,
            state,
            events,
            &client_handshake_response_packet(),
        );
    }

    fn observe_complete_handshake_with_client_packet(
        adapter: &MysqlProtocolAdapter,
        state: &mut dyn ProtocolConnectionState,
        events: &mut VecCaptureEventEmitter,
        client_packet: &[u8],
    ) {
        adapter
            .observe_backend_bytes(state, &initial_handshake_packet(), events)
            .expect("backend handshake should be observed");
        adapter
            .observe_client_bytes(state, client_packet, events)
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
