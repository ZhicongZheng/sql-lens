use serde::{Deserialize, Serialize};

use crate::{
    ConnectionId, DatabaseType, DurationMillis, ErrorSummary, ProtocolMetadata, ProtocolName,
    SqlEventId, StatementId, Timestamp,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SqlEvent {
    pub id: SqlEventId,
    pub timestamp: Timestamp,
    pub protocol: ProtocolName,
    pub database_type: DatabaseType,
    pub connection_id: ConnectionId,
    pub client_addr: String,
    pub backend_addr: String,
    pub user: Option<String>,
    pub database: Option<String>,
    pub kind: SqlEventKind,
    pub status: CaptureStatus,
    pub duration: DurationMillis,
    pub original_sql: String,
    pub normalized_sql: Option<String>,
    pub expanded_sql: Option<String>,
    pub fingerprint: Option<String>,
    pub parameters: Vec<SqlParameter>,
    pub result: Option<ResultSummary>,
    pub error: Option<ErrorSummary>,
    pub timings: QueryTiming,
    pub metadata: ProtocolMetadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SqlEventKind {
    Query,
    StatementPrepare,
    StatementExecute,
    StatementClose,
    ConnectionCommand,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CaptureStatus {
    Ok,
    Slow,
    Error,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectionInfo {
    pub id: ConnectionId,
    pub protocol: ProtocolName,
    pub database_type: DatabaseType,
    pub client_addr: String,
    pub backend_addr: String,
    pub user: Option<String>,
    pub database: Option<String>,
    pub state: ConnectionState,
    pub connected_at: Timestamp,
    pub closed_at: Option<Timestamp>,
    pub last_activity_at: Option<Timestamp>,
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub query_count: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConnectionState {
    Created,
    BackendConnected,
    HandshakeSeen,
    Authenticating,
    Ready,
    CommandInFlight,
    Closing,
    Closed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreparedStatementInfo {
    pub connection_id: ConnectionId,
    pub statement_id: StatementId,
    pub protocol: ProtocolName,
    pub template_sql: String,
    pub parameter_count: u16,
    pub created_at: Timestamp,
    pub closed_at: Option<Timestamp>,
    pub metadata: ProtocolMetadata,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SqlParameter {
    pub index: u16,
    pub name: Option<String>,
    pub value: SqlParameterValue,
    pub redacted: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SqlParameterValue {
    Null,
    Integer(i64),
    Unsigned(u64),
    Float(f64),
    Boolean(bool),
    String(String),
    Date(String),
    Time(String),
    Timestamp(String),
    Json(String),
    BinarySummary(String),
    Unsupported(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryTiming {
    pub started_at: Timestamp,
    pub ended_at: Option<Timestamp>,
    pub duration: DurationMillis,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResultSummary {
    pub affected_rows: Option<u64>,
    pub returned_rows: Option<u64>,
}
