use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use sql_lens_core::{
    CaptureStatus, RedactionPolicy, SqlEvent, SqlEventId, SqlEventKind, SqlParameterValue,
    redact_sql_event,
};

use crate::apply_sqlite_schema;

#[derive(Debug)]
pub struct SqliteEventStore {
    connection: Connection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqliteEventRow {
    pub id: String,
    pub timestamp: String,
    pub target_name: Option<String>,
    pub protocol: String,
    pub database_type: String,
    pub connection_id: String,
    pub client_addr: String,
    pub backend_addr: String,
    pub user: Option<String>,
    pub database: Option<String>,
    pub kind: String,
    pub status: String,
    pub duration_ms: i64,
    pub original_sql: String,
    pub normalized_sql: Option<String>,
    pub expanded_sql: Option<String>,
    pub fingerprint: Option<String>,
    pub affected_rows: Option<i64>,
    pub returned_rows: Option<i64>,
    pub error_code: Option<String>,
    pub error_sql_state: Option<String>,
    pub error_message: Option<String>,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub metadata_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqliteParameterRow {
    pub event_id: String,
    pub parameter_index: i64,
    pub name: Option<String>,
    pub value_type: String,
    pub value_json: String,
    pub redacted: bool,
}

impl SqliteEventStore {
    pub fn new(connection: Connection) -> rusqlite::Result<Self> {
        apply_sqlite_schema(&connection)?;

        Ok(Self { connection })
    }

    pub fn insert_event(&mut self, event: &SqlEvent) -> rusqlite::Result<()> {
        let event = redact_sql_event(event.clone(), &RedactionPolicy::default());
        let metadata_json = serialize_json(&event.metadata)?;
        let affected_rows = event
            .result
            .and_then(|result| result.affected_rows)
            .map(u64_to_i64);
        let returned_rows = event
            .result
            .and_then(|result| result.returned_rows)
            .map(u64_to_i64);
        let error_code = event.error.as_ref().and_then(|error| error.code.as_deref());
        let error_sql_state = event
            .error
            .as_ref()
            .and_then(|error| error.sql_state.as_deref());
        let error_message = event.error.as_ref().map(|error| error.message.as_str());

        let transaction = self.connection.transaction()?;
        transaction.execute(
            r#"
            INSERT INTO sql_events (
                id,
                timestamp,
                target_name,
                protocol,
                database_type,
                connection_id,
                client_addr,
                backend_addr,
                user_name,
                database_name,
                kind,
                status,
                duration_ms,
                original_sql,
                normalized_sql,
                expanded_sql,
                fingerprint,
                affected_rows,
                returned_rows,
                error_code,
                error_sql_state,
                error_message,
                started_at,
                ended_at,
                metadata_json
            )
            VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
                ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20,
                ?21, ?22, ?23, ?24, ?25
            )
            "#,
            params![
                &event.id.0,
                &event.timestamp.0,
                event.target_name.as_deref(),
                &event.protocol.0,
                &event.database_type.0,
                &event.connection_id.0,
                &event.client_addr,
                &event.backend_addr,
                event.user.as_deref(),
                event.database.as_deref(),
                event_kind_name(event.kind),
                capture_status_name(event.status),
                u64_to_i64(event.duration.0),
                &event.original_sql,
                event.normalized_sql.as_deref(),
                event.expanded_sql.as_deref(),
                event.fingerprint.as_deref(),
                affected_rows,
                returned_rows,
                error_code,
                error_sql_state,
                error_message,
                &event.timings.started_at.0,
                event
                    .timings
                    .ended_at
                    .as_ref()
                    .map(|timestamp| timestamp.0.as_str()),
                &metadata_json,
            ],
        )?;

        {
            let mut statement = transaction.prepare(
                r#"
                INSERT INTO sql_parameters (
                    event_id,
                    parameter_index,
                    name,
                    value_type,
                    value_json,
                    redacted
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                "#,
            )?;

            for parameter in &event.parameters {
                let value_json = parameter_value_json(&parameter.value)?;
                statement.execute(params![
                    &event.id.0,
                    i64::from(parameter.index),
                    parameter.name.as_deref(),
                    parameter_value_type(&parameter.value),
                    &value_json,
                    redacted_flag(parameter.redacted),
                ])?;
            }
        }

        transaction.commit()
    }

    pub fn get_event_row(&self, id: &SqlEventId) -> rusqlite::Result<Option<SqliteEventRow>> {
        self.connection
            .query_row(
                r#"
                SELECT
                    id,
                    timestamp,
                    target_name,
                    protocol,
                    database_type,
                    connection_id,
                    client_addr,
                    backend_addr,
                    user_name,
                    database_name,
                    kind,
                    status,
                    duration_ms,
                    original_sql,
                    normalized_sql,
                    expanded_sql,
                    fingerprint,
                    affected_rows,
                    returned_rows,
                    error_code,
                    error_sql_state,
                    error_message,
                    started_at,
                    ended_at,
                    metadata_json
                FROM sql_events
                WHERE id = ?1
                "#,
                params![&id.0],
                event_row_from_sqlite,
            )
            .optional()
    }

    pub fn get_parameter_rows(&self, id: &SqlEventId) -> rusqlite::Result<Vec<SqliteParameterRow>> {
        let mut statement = self.connection.prepare(
            r#"
            SELECT
                event_id,
                parameter_index,
                name,
                value_type,
                value_json,
                redacted
            FROM sql_parameters
            WHERE event_id = ?1
            ORDER BY parameter_index ASC
            "#,
        )?;

        statement
            .query_map(params![&id.0], parameter_row_from_sqlite)?
            .collect()
    }
}

fn event_row_from_sqlite(row: &rusqlite::Row<'_>) -> rusqlite::Result<SqliteEventRow> {
    Ok(SqliteEventRow {
        id: row.get(0)?,
        timestamp: row.get(1)?,
        target_name: row.get(2)?,
        protocol: row.get(3)?,
        database_type: row.get(4)?,
        connection_id: row.get(5)?,
        client_addr: row.get(6)?,
        backend_addr: row.get(7)?,
        user: row.get(8)?,
        database: row.get(9)?,
        kind: row.get(10)?,
        status: row.get(11)?,
        duration_ms: row.get(12)?,
        original_sql: row.get(13)?,
        normalized_sql: row.get(14)?,
        expanded_sql: row.get(15)?,
        fingerprint: row.get(16)?,
        affected_rows: row.get(17)?,
        returned_rows: row.get(18)?,
        error_code: row.get(19)?,
        error_sql_state: row.get(20)?,
        error_message: row.get(21)?,
        started_at: row.get(22)?,
        ended_at: row.get(23)?,
        metadata_json: row.get(24)?,
    })
}

fn parameter_row_from_sqlite(row: &rusqlite::Row<'_>) -> rusqlite::Result<SqliteParameterRow> {
    let redacted: i64 = row.get(5)?;

    Ok(SqliteParameterRow {
        event_id: row.get(0)?,
        parameter_index: row.get(1)?,
        name: row.get(2)?,
        value_type: row.get(3)?,
        value_json: row.get(4)?,
        redacted: redacted != 0,
    })
}

fn serialize_json<T: Serialize>(value: &T) -> rusqlite::Result<String> {
    serde_json::to_string(value).map_err(json_to_sqlite_error)
}

fn parameter_value_json(value: &SqlParameterValue) -> rusqlite::Result<String> {
    match value {
        SqlParameterValue::Null => Ok("null".to_owned()),
        SqlParameterValue::Integer(value) => serialize_json(value),
        SqlParameterValue::Unsigned(value) => serialize_json(value),
        SqlParameterValue::Float(value) => serialize_json(value),
        SqlParameterValue::Boolean(value) => serialize_json(value),
        SqlParameterValue::String(value)
        | SqlParameterValue::Date(value)
        | SqlParameterValue::Time(value)
        | SqlParameterValue::Timestamp(value)
        | SqlParameterValue::Json(value)
        | SqlParameterValue::BinarySummary(value)
        | SqlParameterValue::Unsupported(value) => serialize_json(value),
    }
}

fn json_to_sqlite_error(error: serde_json::Error) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(error))
}

fn u64_to_i64(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn redacted_flag(redacted: bool) -> i64 {
    i64::from(redacted)
}

fn event_kind_name(kind: SqlEventKind) -> &'static str {
    match kind {
        SqlEventKind::Query => "query",
        SqlEventKind::StatementPrepare => "statement_prepare",
        SqlEventKind::StatementExecute => "statement_execute",
        SqlEventKind::StatementClose => "statement_close",
        SqlEventKind::ConnectionCommand => "connection_command",
        SqlEventKind::Unknown => "unknown",
    }
}

fn capture_status_name(status: CaptureStatus) -> &'static str {
    match status {
        CaptureStatus::Ok => "ok",
        CaptureStatus::Slow => "slow",
        CaptureStatus::Error => "error",
        CaptureStatus::Unknown => "unknown",
    }
}

fn parameter_value_type(value: &SqlParameterValue) -> &'static str {
    match value {
        SqlParameterValue::Null => "null",
        SqlParameterValue::Integer(_) => "integer",
        SqlParameterValue::Unsigned(_) => "unsigned",
        SqlParameterValue::Float(_) => "float",
        SqlParameterValue::Boolean(_) => "boolean",
        SqlParameterValue::String(_) => "string",
        SqlParameterValue::Date(_) => "date",
        SqlParameterValue::Time(_) => "time",
        SqlParameterValue::Timestamp(_) => "timestamp",
        SqlParameterValue::Json(_) => "json",
        SqlParameterValue::BinarySummary(_) => "binary_summary",
        SqlParameterValue::Unsupported(_) => "unsupported",
    }
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;
    use serde_json::Value;
    use sql_lens_core::{
        CaptureStatus, ConnectionId, DatabaseType, DurationMillis, ErrorSummary, MetadataField,
        MetadataValue, ProtocolMetadata, ProtocolName, QueryTiming, ResultSummary, SqlEvent,
        SqlEventId, SqlEventKind, SqlParameter, SqlParameterValue, Timestamp,
    };

    use super::*;

    #[test]
    fn sqlite_event_store_inserts_and_reads_event_scalars() {
        let mut store = in_memory_store();
        let event = test_event("evt_1");

        store
            .insert_event(&event)
            .expect("event insert should succeed");

        let row = store
            .get_event_row(&event.id)
            .expect("event lookup should succeed")
            .expect("event should exist");

        assert_eq!(row.id, "evt_1");
        assert_eq!(row.timestamp, "2026-07-09T08:00:00Z");
        assert_eq!(row.target_name, Some("mysql-local".to_owned()));
        assert_eq!(row.protocol, "mysql");
        assert_eq!(row.database_type, "mysql");
        assert_eq!(row.connection_id, "conn_1");
        assert_eq!(row.client_addr, "127.0.0.1:51000");
        assert_eq!(row.backend_addr, "127.0.0.1:3306");
        assert_eq!(row.user, Some("app".to_owned()));
        assert_eq!(row.database, Some("orders".to_owned()));
        assert_eq!(row.kind, "query");
        assert_eq!(row.status, "ok");
        assert_eq!(row.duration_ms, 12);
        assert_eq!(row.original_sql, "SELECT * FROM users WHERE id = ?");
        assert_eq!(
            row.normalized_sql,
            Some("select * from users where id = ?".to_owned())
        );
        assert_eq!(
            row.expanded_sql,
            Some("SELECT * FROM users WHERE id = 42".to_owned())
        );
        assert_eq!(
            row.fingerprint,
            Some("select * from users where id = ?".to_owned())
        );
        assert_eq!(row.affected_rows, Some(1));
        assert_eq!(row.returned_rows, Some(3));
        assert_eq!(row.error_code, Some("ER_TEST".to_owned()));
        assert_eq!(row.error_sql_state, Some("HY000".to_owned()));
        assert_eq!(row.error_message, Some("synthetic error".to_owned()));
        assert_eq!(row.started_at, "2026-07-09T08:00:00Z");
        assert_eq!(row.ended_at, Some("2026-07-09T08:00:00.012Z".to_owned()));

        let metadata: Value =
            serde_json::from_str(&row.metadata_json).expect("metadata should be JSON");
        assert_eq!(metadata["protocol"], "mysql");
        assert_eq!(metadata["fields"][0]["key"], "command");
    }

    #[test]
    fn sqlite_event_store_inserts_parameter_rows() {
        let mut store = in_memory_store();
        let event = test_event("evt_params");

        store
            .insert_event(&event)
            .expect("event insert should succeed");

        let parameters = store
            .get_parameter_rows(&event.id)
            .expect("parameter lookup should succeed");

        assert_eq!(
            parameters,
            vec![
                SqliteParameterRow {
                    event_id: "evt_params".to_owned(),
                    parameter_index: 0,
                    name: Some("id".to_owned()),
                    value_type: "integer".to_owned(),
                    value_json: "42".to_owned(),
                    redacted: false,
                },
                SqliteParameterRow {
                    event_id: "evt_params".to_owned(),
                    parameter_index: 1,
                    name: Some("active".to_owned()),
                    value_type: "boolean".to_owned(),
                    value_json: "true".to_owned(),
                    redacted: false,
                },
            ]
        );
    }

    #[test]
    fn sqlite_event_store_redacts_before_persisting() {
        let mut store = in_memory_store();
        let mut event = test_event("evt_secret");
        event.original_sql = "SELECT * FROM users WHERE password = 'secret'".to_owned();
        event.expanded_sql = Some("SELECT * FROM users WHERE password = 'secret'".to_owned());
        event.parameters = vec![SqlParameter {
            index: 0,
            name: Some("password".to_owned()),
            value: SqlParameterValue::String("secret".to_owned()),
            redacted: false,
        }];

        store
            .insert_event(&event)
            .expect("event insert should succeed");

        let row = store
            .get_event_row(&event.id)
            .expect("event lookup should succeed")
            .expect("event should exist");
        let parameters = store
            .get_parameter_rows(&event.id)
            .expect("parameter lookup should succeed");

        assert_eq!(
            row.original_sql,
            "SELECT * FROM users WHERE password = '***'"
        );
        assert_eq!(
            row.expanded_sql,
            Some("SELECT * FROM users WHERE password = '***'".to_owned())
        );
        assert_eq!(parameters[0].value_json, "\"***\"");
        assert!(parameters[0].redacted);
    }

    #[test]
    fn sqlite_event_store_rejects_duplicate_event_ids() {
        let mut store = in_memory_store();
        let event = test_event("evt_duplicate");

        store
            .insert_event(&event)
            .expect("first event insert should succeed");
        let error = store
            .insert_event(&event)
            .expect_err("duplicate event IDs should fail");

        assert!(matches!(
            error,
            rusqlite::Error::SqliteFailure(_, Some(message))
                if message.contains("UNIQUE constraint failed")
        ));
    }

    #[test]
    fn sqlite_event_store_returns_none_for_missing_event() {
        let store = in_memory_store();

        let row = store
            .get_event_row(&SqlEventId("missing".to_owned()))
            .expect("event lookup should succeed");

        assert_eq!(row, None);
    }

    fn in_memory_store() -> SqliteEventStore {
        let connection = Connection::open_in_memory().expect("in-memory database should open");

        SqliteEventStore::new(connection).expect("sqlite store should initialize")
    }

    fn test_event(id: &str) -> SqlEvent {
        SqlEvent {
            id: SqlEventId(id.to_owned()),
            timestamp: Timestamp("2026-07-09T08:00:00Z".to_owned()),
            target_name: Some("mysql-local".to_owned()),
            protocol: ProtocolName("mysql".to_owned()),
            database_type: DatabaseType("mysql".to_owned()),
            connection_id: ConnectionId("conn_1".to_owned()),
            client_addr: "127.0.0.1:51000".to_owned(),
            backend_addr: "127.0.0.1:3306".to_owned(),
            user: Some("app".to_owned()),
            database: Some("orders".to_owned()),
            kind: SqlEventKind::Query,
            status: CaptureStatus::Ok,
            duration: DurationMillis(12),
            original_sql: "SELECT * FROM users WHERE id = ?".to_owned(),
            normalized_sql: Some("select * from users where id = ?".to_owned()),
            expanded_sql: Some("SELECT * FROM users WHERE id = 42".to_owned()),
            fingerprint: Some("select * from users where id = ?".to_owned()),
            parameters: vec![
                SqlParameter {
                    index: 0,
                    name: Some("id".to_owned()),
                    value: SqlParameterValue::Integer(42),
                    redacted: false,
                },
                SqlParameter {
                    index: 1,
                    name: Some("active".to_owned()),
                    value: SqlParameterValue::Boolean(true),
                    redacted: false,
                },
            ],
            result: Some(ResultSummary {
                affected_rows: Some(1),
                returned_rows: Some(3),
            }),
            error: Some(ErrorSummary {
                code: Some("ER_TEST".to_owned()),
                sql_state: Some("HY000".to_owned()),
                message: "synthetic error".to_owned(),
                metadata: None,
            }),
            timings: QueryTiming {
                started_at: Timestamp("2026-07-09T08:00:00Z".to_owned()),
                ended_at: Some(Timestamp("2026-07-09T08:00:00.012Z".to_owned())),
                duration: DurationMillis(12),
            },
            metadata: ProtocolMetadata {
                protocol: ProtocolName("mysql".to_owned()),
                fields: vec![MetadataField {
                    key: "command".to_owned(),
                    value: MetadataValue::String("COM_QUERY".to_owned()),
                }],
            },
        }
    }
}
