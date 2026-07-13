use std::{error::Error, fmt, num::NonZeroUsize, path::Path};

use rusqlite::{Connection, OptionalExtension, ToSql, params, params_from_iter};
use serde::Serialize;
use sql_lens_core::{
    CaptureStatus, DatabaseType, DurationMillis, ProtocolName, RedactionPolicy, SqlEvent,
    SqlEventId, SqlEventKind, SqlParameterValue, Timestamp, redact_sql_event,
};

use crate::{SqlEventFilter, SqlEventFilterError, apply_sqlite_schema};

#[derive(Debug)]
pub struct SqliteEventStore {
    connection: Connection,
    redaction_policy: RedactionPolicy,
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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SqliteRetentionOutcome {
    pub deleted_event_ids: Vec<SqlEventId>,
    pub deleted_event_count: usize,
    pub deleted_parameter_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqliteTimelineQuery {
    pub limit: NonZeroUsize,
    pub cursor: Option<SqliteTimelineCursor>,
    pub filter: SqlEventFilter,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqliteTimelineCursor {
    pub before_timestamp: Timestamp,
    pub before_event_id: SqlEventId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqliteTimelinePage {
    pub events: Vec<SqliteEventRow>,
    pub next_cursor: Option<SqliteTimelineCursor>,
}

#[derive(Debug)]
pub enum SqliteTimelineQueryError {
    InvalidFilter(SqlEventFilterError),
    Sqlite(rusqlite::Error),
}

impl fmt::Display for SqliteTimelineQueryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFilter(error) => write!(f, "{error}"),
            Self::Sqlite(error) => write!(f, "sqlite timeline query failed: {error}"),
        }
    }
}

impl Error for SqliteTimelineQueryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidFilter(error) => Some(error),
            Self::Sqlite(error) => Some(error),
        }
    }
}

impl From<SqlEventFilterError> for SqliteTimelineQueryError {
    fn from(error: SqlEventFilterError) -> Self {
        Self::InvalidFilter(error)
    }
}

impl From<rusqlite::Error> for SqliteTimelineQueryError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sqlite(error)
    }
}

impl SqliteEventStore {
    pub fn open(path: impl AsRef<Path>) -> rusqlite::Result<Self> {
        Self::open_with_redaction_policy(path, RedactionPolicy::default())
    }

    pub fn open_with_redaction_policy(
        path: impl AsRef<Path>,
        redaction_policy: RedactionPolicy,
    ) -> rusqlite::Result<Self> {
        Self::new_with_redaction_policy(Connection::open(path)?, redaction_policy)
    }

    pub fn new(connection: Connection) -> rusqlite::Result<Self> {
        Self::new_with_redaction_policy(connection, RedactionPolicy::default())
    }

    pub fn new_with_redaction_policy(
        connection: Connection,
        redaction_policy: RedactionPolicy,
    ) -> rusqlite::Result<Self> {
        apply_sqlite_schema(&connection)?;

        Ok(Self {
            connection,
            redaction_policy,
        })
    }

    pub fn insert_event(&mut self, event: &SqlEvent) -> rusqlite::Result<()> {
        let event = redact_sql_event(event.clone(), &self.redaction_policy);
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

    pub fn delete_events_older_than(
        &mut self,
        cutoff: &Timestamp,
    ) -> rusqlite::Result<SqliteRetentionOutcome> {
        let transaction = self.connection.transaction()?;
        let deleted_event_ids = {
            let mut statement = transaction.prepare(
                r#"
                SELECT id
                FROM sql_events
                WHERE timestamp < ?1
                ORDER BY timestamp ASC, id ASC
                "#,
            )?;

            statement
                .query_map(params![&cutoff.0], |row| {
                    row.get::<_, String>(0).map(SqlEventId)
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?
        };

        let outcome = delete_sqlite_events(&transaction, deleted_event_ids)?;
        transaction.commit()?;

        Ok(outcome)
    }

    pub fn enforce_max_events(
        &mut self,
        max_events: NonZeroUsize,
    ) -> rusqlite::Result<SqliteRetentionOutcome> {
        let transaction = self.connection.transaction()?;
        let event_count = sqlite_event_count(&transaction)?;
        let max_events = max_events.get();

        if event_count <= max_events {
            transaction.commit()?;
            return Ok(SqliteRetentionOutcome::default());
        }

        let delete_count = event_count - max_events;
        let deleted_event_ids = {
            let mut statement = transaction.prepare(
                r#"
                SELECT id
                FROM sql_events
                ORDER BY timestamp ASC, id ASC
                LIMIT ?1
                "#,
            )?;

            statement
                .query_map(params![usize_to_i64(delete_count)], |row| {
                    row.get::<_, String>(0).map(SqlEventId)
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?
        };

        let outcome = delete_sqlite_events(&transaction, deleted_event_ids)?;
        transaction.commit()?;

        Ok(outcome)
    }

    pub fn query_timeline(
        &self,
        query: SqliteTimelineQuery,
    ) -> Result<SqliteTimelinePage, SqliteTimelineQueryError> {
        query.filter.validate()?;

        let limit = query.limit.get();
        let fetch_limit = limit.saturating_add(1);
        let mut sql = String::from(
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
            "#,
        );
        let mut predicates = Vec::new();
        let mut parameters: Vec<Box<dyn ToSql>> = Vec::new();

        push_cursor_predicate(&mut predicates, &mut parameters, query.cursor);
        push_filter_predicates(&mut predicates, &mut parameters, &query.filter);

        if !predicates.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&predicates.join(" AND "));
        }

        sql.push_str(" ORDER BY timestamp DESC, id DESC LIMIT ?");
        parameters.push(Box::new(usize_to_i64(fetch_limit)));

        let mut statement = self.connection.prepare(&sql)?;
        let mut events = statement
            .query_map(
                params_from_iter(parameters.iter().map(|parameter| parameter.as_ref())),
                event_row_from_sqlite,
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        let has_more_older_events = events.len() > limit;
        if has_more_older_events {
            events.truncate(limit);
        }

        let next_cursor = if has_more_older_events {
            events.last().map(|event| SqliteTimelineCursor {
                before_timestamp: Timestamp(event.timestamp.clone()),
                before_event_id: SqlEventId(event.id.clone()),
            })
        } else {
            None
        };

        Ok(SqliteTimelinePage {
            events,
            next_cursor,
        })
    }
}

fn sqlite_event_count(connection: &Connection) -> rusqlite::Result<usize> {
    let count = connection.query_row("SELECT COUNT(*) FROM sql_events", [], |row| {
        row.get::<_, i64>(0)
    })?;

    Ok(usize::try_from(count).unwrap_or(usize::MAX))
}

fn delete_sqlite_events(
    connection: &Connection,
    deleted_event_ids: Vec<SqlEventId>,
) -> rusqlite::Result<SqliteRetentionOutcome> {
    if deleted_event_ids.is_empty() {
        return Ok(SqliteRetentionOutcome::default());
    }

    let mut deleted_parameter_count = 0;
    let mut deleted_event_count = 0;

    for event_id in &deleted_event_ids {
        deleted_parameter_count += connection.execute(
            "DELETE FROM sql_parameters WHERE event_id = ?1",
            params![&event_id.0],
        )?;
        deleted_event_count +=
            connection.execute("DELETE FROM sql_events WHERE id = ?1", params![&event_id.0])?;
    }

    Ok(SqliteRetentionOutcome {
        deleted_event_ids,
        deleted_event_count,
        deleted_parameter_count,
    })
}

fn push_cursor_predicate(
    predicates: &mut Vec<&'static str>,
    parameters: &mut Vec<Box<dyn ToSql>>,
    cursor: Option<SqliteTimelineCursor>,
) {
    if let Some(cursor) = cursor {
        predicates.push("((timestamp < ?) OR (timestamp = ? AND id < ?))");
        parameters.push(Box::new(cursor.before_timestamp.0.clone()));
        parameters.push(Box::new(cursor.before_timestamp.0));
        parameters.push(Box::new(cursor.before_event_id.0));
    }
}

fn push_filter_predicates(
    predicates: &mut Vec<&'static str>,
    parameters: &mut Vec<Box<dyn ToSql>>,
    filter: &SqlEventFilter,
) {
    push_optional_string_filter(
        predicates,
        parameters,
        "target_name = ?",
        filter.target_name.as_deref(),
    );
    push_optional_protocol_filter(
        predicates,
        parameters,
        "protocol = ?",
        filter.protocol.as_ref(),
    );
    push_optional_database_type_filter(
        predicates,
        parameters,
        "database_type = ?",
        filter.database_type.as_ref(),
    );
    push_optional_string_filter(
        predicates,
        parameters,
        "database_name = ?",
        filter.database.as_deref(),
    );
    push_optional_string_filter(
        predicates,
        parameters,
        "user_name = ?",
        filter.user.as_deref(),
    );
    push_optional_string_filter(
        predicates,
        parameters,
        "client_addr = ?",
        filter.client_addr.as_deref(),
    );

    if let Some(status) = filter.status {
        predicates.push("status = ?");
        parameters.push(Box::new(capture_status_name(status).to_owned()));
    }

    if let Some(min_duration) = filter.min_duration {
        predicates.push("duration_ms >= ?");
        parameters.push(Box::new(duration_millis_to_i64(min_duration)));
    }

    if let Some(max_duration) = filter.max_duration {
        predicates.push("duration_ms <= ?");
        parameters.push(Box::new(duration_millis_to_i64(max_duration)));
    }

    if let Some(text) = filter.text.as_deref() {
        predicates.push(
            "(instr(original_sql, ?) > 0 OR instr(COALESCE(normalized_sql, ''), ?) > 0 OR instr(COALESCE(expanded_sql, ''), ?) > 0)",
        );
        parameters.push(Box::new(text.to_owned()));
        parameters.push(Box::new(text.to_owned()));
        parameters.push(Box::new(text.to_owned()));
    }

    push_optional_string_filter(
        predicates,
        parameters,
        "fingerprint = ?",
        filter.fingerprint.as_deref(),
    );

    if let Some(from) = &filter.from {
        predicates.push("timestamp >= ?");
        parameters.push(Box::new(from.0.clone()));
    }

    if let Some(to) = &filter.to {
        predicates.push("timestamp <= ?");
        parameters.push(Box::new(to.0.clone()));
    }
}

fn push_optional_string_filter(
    predicates: &mut Vec<&'static str>,
    parameters: &mut Vec<Box<dyn ToSql>>,
    predicate: &'static str,
    value: Option<&str>,
) {
    if let Some(value) = value {
        predicates.push(predicate);
        parameters.push(Box::new(value.to_owned()));
    }
}

fn push_optional_protocol_filter(
    predicates: &mut Vec<&'static str>,
    parameters: &mut Vec<Box<dyn ToSql>>,
    predicate: &'static str,
    value: Option<&ProtocolName>,
) {
    if let Some(value) = value {
        predicates.push(predicate);
        parameters.push(Box::new(value.0.clone()));
    }
}

fn push_optional_database_type_filter(
    predicates: &mut Vec<&'static str>,
    parameters: &mut Vec<Box<dyn ToSql>>,
    predicate: &'static str,
    value: Option<&DatabaseType>,
) {
    if let Some(value) = value {
        predicates.push(predicate);
        parameters.push(Box::new(value.0.clone()));
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

fn usize_to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn duration_millis_to_i64(value: DurationMillis) -> i64 {
    u64_to_i64(value.0)
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
    use std::{
        num::NonZeroUsize,
        time::{SystemTime, UNIX_EPOCH},
    };

    use rusqlite::Connection;
    use serde_json::Value;
    use sql_lens_core::{
        CaptureStatus, ConnectionId, DatabaseType, DurationMillis, ErrorSummary, MetadataField,
        MetadataValue, ProtocolMetadata, ProtocolName, QueryTiming, RedactionPolicy, ResultSummary,
        SqlEvent, SqlEventId, SqlEventKind, SqlParameter, SqlParameterValue, Timestamp,
    };

    use super::*;

    #[test]
    fn sqlite_event_store_opens_file_path_and_applies_schema() {
        let path = temporary_sqlite_path("open-file");
        let store = SqliteEventStore::open(&path).expect("sqlite file store should open");
        drop(store);

        let connection = Connection::open(&path).expect("sqlite file should reopen");
        let schema_version: i64 = connection
            .query_row("SELECT version FROM schema_version", [], |row| row.get(0))
            .expect("schema version row should exist");

        assert_eq!(schema_version, crate::SQLITE_SCHEMA_VERSION);
        let _ = std::fs::remove_file(path);
    }

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
    fn sqlite_event_store_uses_configured_redaction_policy() {
        let connection = Connection::open_in_memory().expect("in-memory database should open");
        let policy = RedactionPolicy {
            mask: "[MASK]".to_owned(),
            parameter_names: vec!["password".to_owned()],
            sql_patterns: vec!["secret_value".to_owned()],
            ..RedactionPolicy::default()
        };
        let mut store = SqliteEventStore::new_with_redaction_policy(connection, policy)
            .expect("sqlite store should initialize");
        let mut event = test_event("evt_custom_policy");
        event.original_sql = "SELECT secret_value FROM users WHERE password = ?".to_owned();
        event.expanded_sql =
            Some("SELECT secret_value FROM users WHERE password = 'top-secret'".to_owned());
        event.parameters = vec![SqlParameter {
            index: 0,
            name: Some("password".to_owned()),
            value: SqlParameterValue::String("top-secret".to_owned()),
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
            "SELECT [MASK] FROM users WHERE password = ?"
        );
        assert_eq!(
            row.expanded_sql,
            Some("SELECT [MASK] FROM users WHERE password = '[MASK]'".to_owned())
        );
        assert_eq!(parameters[0].value_json, "\"[MASK]\"");
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

    #[test]
    fn sqlite_timeline_returns_newest_events_first() {
        let mut store = in_memory_store();
        insert_events(
            &mut store,
            vec![
                event_at("evt_1", "2026-07-09T08:00:00Z"),
                event_at("evt_2", "2026-07-09T08:01:00Z"),
                event_at("evt_3", "2026-07-09T08:02:00Z"),
            ],
        );

        let page = query_page(&store, timeline_query(3, None));

        assert_eq!(row_ids(&page.events), ["evt_3", "evt_2", "evt_1"]);
        assert_eq!(page.next_cursor, None);
    }

    #[test]
    fn sqlite_timeline_uses_event_id_to_order_equal_timestamps() {
        let mut store = in_memory_store();
        insert_events(
            &mut store,
            vec![
                event_at("evt_a", "2026-07-09T08:00:00Z"),
                event_at("evt_c", "2026-07-09T08:00:00Z"),
                event_at("evt_b", "2026-07-09T08:00:00Z"),
            ],
        );

        let page = query_page(&store, timeline_query(3, None));

        assert_eq!(row_ids(&page.events), ["evt_c", "evt_b", "evt_a"]);
    }

    #[test]
    fn sqlite_timeline_cursor_pages_older_events_without_duplicates() {
        let mut store = in_memory_store();
        insert_events(
            &mut store,
            vec![
                event_at("evt_1", "2026-07-09T08:00:00Z"),
                event_at("evt_2", "2026-07-09T08:01:00Z"),
                event_at("evt_3", "2026-07-09T08:02:00Z"),
                event_at("evt_4", "2026-07-09T08:03:00Z"),
                event_at("evt_5", "2026-07-09T08:04:00Z"),
            ],
        );

        let first_page = query_page(&store, timeline_query(2, None));
        let second_page = query_page(&store, timeline_query(2, first_page.next_cursor.clone()));
        let third_page = query_page(&store, timeline_query(2, second_page.next_cursor.clone()));

        assert_eq!(row_ids(&first_page.events), ["evt_5", "evt_4"]);
        assert_eq!(row_ids(&second_page.events), ["evt_3", "evt_2"]);
        assert_eq!(row_ids(&third_page.events), ["evt_1"]);
        assert!(first_page.next_cursor.is_some());
        assert!(second_page.next_cursor.is_some());
        assert_eq!(third_page.next_cursor, None);
    }

    #[test]
    fn sqlite_timeline_cursor_is_stable_after_newer_insert() {
        let mut store = in_memory_store();
        insert_events(
            &mut store,
            vec![
                event_at("evt_1", "2026-07-09T08:00:00Z"),
                event_at("evt_2", "2026-07-09T08:01:00Z"),
                event_at("evt_3", "2026-07-09T08:02:00Z"),
                event_at("evt_4", "2026-07-09T08:03:00Z"),
            ],
        );

        let first_page = query_page(&store, timeline_query(2, None));
        store
            .insert_event(&event_at("evt_5", "2026-07-09T08:04:00Z"))
            .expect("newer event insert should succeed");
        let second_page = query_page(&store, timeline_query(2, first_page.next_cursor.clone()));

        assert_eq!(row_ids(&first_page.events), ["evt_4", "evt_3"]);
        assert_eq!(row_ids(&second_page.events), ["evt_2", "evt_1"]);
        assert_eq!(second_page.next_cursor, None);
    }

    #[test]
    fn sqlite_timeline_filters_by_common_indexed_fields() {
        let mut store = in_memory_store();
        let mut target = event_at("evt_target", "2026-07-09T08:00:00Z");
        target.target_name = Some("starrocks-local".to_owned());
        target.protocol = ProtocolName("mysql".to_owned());
        target.database_type = DatabaseType("starrocks".to_owned());
        target.database = Some("analytics".to_owned());
        target.user = Some("analyst".to_owned());
        target.status = CaptureStatus::Error;

        let mut wrong_target = target.clone();
        wrong_target.id = SqlEventId("evt_wrong_target".to_owned());
        wrong_target.target_name = Some("mysql-local".to_owned());
        let mut wrong_database = target.clone();
        wrong_database.id = SqlEventId("evt_wrong_database".to_owned());
        wrong_database.database = Some("ops".to_owned());
        let mut wrong_user = target.clone();
        wrong_user.id = SqlEventId("evt_wrong_user".to_owned());
        wrong_user.user = Some("app".to_owned());
        let mut wrong_status = target.clone();
        wrong_status.id = SqlEventId("evt_wrong_status".to_owned());
        wrong_status.status = CaptureStatus::Ok;

        insert_events(
            &mut store,
            vec![
                target,
                wrong_target,
                wrong_database,
                wrong_user,
                wrong_status,
            ],
        );

        let page = query_page(
            &store,
            filtered_timeline_query(
                10,
                None,
                SqlEventFilter {
                    target_name: Some("starrocks-local".to_owned()),
                    protocol: Some(ProtocolName("mysql".to_owned())),
                    database_type: Some(DatabaseType("starrocks".to_owned())),
                    database: Some("analytics".to_owned()),
                    user: Some("analyst".to_owned()),
                    status: Some(CaptureStatus::Error),
                    ..SqlEventFilter::default()
                },
            ),
        );

        assert_eq!(row_ids(&page.events), ["evt_target"]);
        assert_eq!(page.next_cursor, None);
    }

    #[test]
    fn sqlite_timeline_filters_by_duration_timestamp_text_and_fingerprint() {
        let mut store = in_memory_store();
        let mut target = event_at("evt_target", "2026-07-09T08:05:00Z");
        target.duration = DurationMillis(7);
        target.original_sql = "SELECT * FROM invoices WHERE id = ?".to_owned();
        target.normalized_sql = Some("select * from orders where id = ?".to_owned());
        target.expanded_sql = None;
        target.fingerprint = Some("select * from orders where id = ?".to_owned());

        let mut wrong_duration = target.clone();
        wrong_duration.id = SqlEventId("evt_wrong_duration".to_owned());
        wrong_duration.duration = DurationMillis(20);
        let mut wrong_timestamp = target.clone();
        wrong_timestamp.id = SqlEventId("evt_wrong_timestamp".to_owned());
        wrong_timestamp.timestamp = Timestamp("2026-07-09T08:20:00Z".to_owned());
        let mut wrong_text = target.clone();
        wrong_text.id = SqlEventId("evt_wrong_text".to_owned());
        wrong_text.normalized_sql = Some("select * from invoices where id = ?".to_owned());
        let mut wrong_fingerprint = target.clone();
        wrong_fingerprint.id = SqlEventId("evt_wrong_fingerprint".to_owned());
        wrong_fingerprint.fingerprint = Some("select * from users where id = ?".to_owned());

        insert_events(
            &mut store,
            vec![
                target,
                wrong_duration,
                wrong_timestamp,
                wrong_text,
                wrong_fingerprint,
            ],
        );

        let page = query_page(
            &store,
            filtered_timeline_query(
                10,
                None,
                SqlEventFilter {
                    min_duration: Some(DurationMillis(2)),
                    max_duration: Some(DurationMillis(8)),
                    text: Some("orders".to_owned()),
                    fingerprint: Some("select * from orders where id = ?".to_owned()),
                    from: Some(Timestamp("2026-07-09T08:01:00Z".to_owned())),
                    to: Some(Timestamp("2026-07-09T08:09:00Z".to_owned())),
                    ..SqlEventFilter::default()
                },
            ),
        );

        assert_eq!(row_ids(&page.events), ["evt_target"]);
        assert_eq!(page.next_cursor, None);
    }

    #[test]
    fn sqlite_timeline_rejects_invalid_duration_range() {
        let store = in_memory_store();

        let error = store
            .query_timeline(filtered_timeline_query(
                10,
                None,
                SqlEventFilter {
                    min_duration: Some(DurationMillis(10)),
                    max_duration: Some(DurationMillis(5)),
                    ..SqlEventFilter::default()
                },
            ))
            .expect_err("invalid duration range should fail");

        assert!(matches!(
            error,
            SqliteTimelineQueryError::InvalidFilter(SqlEventFilterError::InvalidDurationRange {
                min: DurationMillis(10),
                max: DurationMillis(5),
            })
        ));
        assert!(!error.to_string().is_empty());
        assert!(error.source().is_some());
    }

    #[test]
    fn sqlite_timeline_rejects_invalid_timestamp_range() {
        let store = in_memory_store();

        let error = store
            .query_timeline(filtered_timeline_query(
                10,
                None,
                SqlEventFilter {
                    from: Some(Timestamp("2026-07-09T08:10:00Z".to_owned())),
                    to: Some(Timestamp("2026-07-09T08:00:00Z".to_owned())),
                    ..SqlEventFilter::default()
                },
            ))
            .expect_err("invalid timestamp range should fail");

        assert!(matches!(
            error,
            SqliteTimelineQueryError::InvalidFilter(
                SqlEventFilterError::InvalidTimestampRange { .. }
            )
        ));
        assert!(!error.to_string().is_empty());
        assert!(error.source().is_some());
    }

    #[test]
    fn sqlite_timeline_returns_empty_page_without_next_cursor() {
        let store = in_memory_store();

        let page = query_page(&store, timeline_query(10, None));

        assert!(page.events.is_empty());
        assert_eq!(page.next_cursor, None);
    }

    #[test]
    fn sqlite_retention_deletes_events_older_than_cutoff() {
        let mut store = in_memory_store();
        insert_events(
            &mut store,
            vec![
                event_at("evt_oldest", "2026-07-09T08:00:00Z"),
                event_at("evt_old", "2026-07-09T08:01:00Z"),
                event_at("evt_keep", "2026-07-09T08:02:00Z"),
            ],
        );

        let outcome = store
            .delete_events_older_than(&Timestamp("2026-07-09T08:02:00Z".to_owned()))
            .expect("age cleanup should succeed");

        assert_eq!(
            outcome.deleted_event_ids,
            vec![
                SqlEventId("evt_oldest".to_owned()),
                SqlEventId("evt_old".to_owned())
            ]
        );
        assert_eq!(outcome.deleted_event_count, 2);
        assert_eq!(outcome.deleted_parameter_count, 4);
        assert_eq!(
            row_ids(&query_page(&store, timeline_query(10, None)).events),
            ["evt_keep"]
        );
    }

    #[test]
    fn sqlite_retention_enforces_max_events_with_timeline_ordering() {
        let mut store = in_memory_store();
        insert_events(
            &mut store,
            vec![
                event_at("evt_a", "2026-07-09T08:00:00Z"),
                event_at("evt_b", "2026-07-09T08:00:00Z"),
                event_at("evt_c", "2026-07-09T08:00:00Z"),
                event_at("evt_new", "2026-07-09T08:01:00Z"),
            ],
        );

        let outcome = store
            .enforce_max_events(NonZeroUsize::new(2).expect("max events should be non-zero"))
            .expect("count cleanup should succeed");

        assert_eq!(
            outcome.deleted_event_ids,
            vec![
                SqlEventId("evt_a".to_owned()),
                SqlEventId("evt_b".to_owned())
            ]
        );
        assert_eq!(outcome.deleted_event_count, 2);
        assert_eq!(outcome.deleted_parameter_count, 4);
        assert_eq!(
            row_ids(&query_page(&store, timeline_query(10, None)).events),
            ["evt_new", "evt_c"]
        );
    }

    #[test]
    fn sqlite_retention_deletes_parameter_rows_for_deleted_events() {
        let mut store = in_memory_store();
        let deleted = event_at("evt_deleted", "2026-07-09T08:00:00Z");
        let kept = event_at("evt_kept", "2026-07-09T08:01:00Z");
        insert_events(&mut store, vec![deleted.clone(), kept.clone()]);

        store
            .delete_events_older_than(&Timestamp("2026-07-09T08:01:00Z".to_owned()))
            .expect("age cleanup should succeed");

        assert_eq!(
            store
                .get_parameter_rows(&deleted.id)
                .expect("deleted parameters should be readable"),
            Vec::new()
        );
        assert_eq!(
            store
                .get_parameter_rows(&kept.id)
                .expect("kept parameters should be readable")
                .len(),
            2
        );
        assert_eq!(
            store
                .get_event_row(&deleted.id)
                .expect("deleted event lookup should succeed"),
            None
        );
    }

    #[test]
    fn sqlite_retention_noops_when_nothing_matches() {
        let mut store = in_memory_store();
        insert_events(
            &mut store,
            vec![
                event_at("evt_1", "2026-07-09T08:00:00Z"),
                event_at("evt_2", "2026-07-09T08:01:00Z"),
            ],
        );

        let age_outcome = store
            .delete_events_older_than(&Timestamp("2026-07-09T07:00:00Z".to_owned()))
            .expect("age cleanup should succeed");
        let count_outcome = store
            .enforce_max_events(NonZeroUsize::new(2).expect("max events should be non-zero"))
            .expect("count cleanup should succeed");

        assert_eq!(age_outcome, SqliteRetentionOutcome::default());
        assert_eq!(count_outcome, SqliteRetentionOutcome::default());
        assert_eq!(
            row_ids(&query_page(&store, timeline_query(10, None)).events),
            ["evt_2", "evt_1"]
        );
    }

    fn in_memory_store() -> SqliteEventStore {
        let connection = Connection::open_in_memory().expect("in-memory database should open");

        SqliteEventStore::new(connection).expect("sqlite store should initialize")
    }

    fn temporary_sqlite_path(name: &str) -> std::path::PathBuf {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_millis();

        std::env::temp_dir().join(format!(
            "sql-lens-storage-{name}-{}-{millis}.sqlite3",
            std::process::id()
        ))
    }

    fn insert_events(store: &mut SqliteEventStore, events: Vec<SqlEvent>) {
        for event in events {
            store
                .insert_event(&event)
                .expect("event insert should succeed");
        }
    }

    fn event_at(id: &str, timestamp: &str) -> SqlEvent {
        let mut event = test_event(id);
        event.timestamp = Timestamp(timestamp.to_owned());
        event.timings.started_at = Timestamp(timestamp.to_owned());
        event
    }

    fn timeline_query(limit: usize, cursor: Option<SqliteTimelineCursor>) -> SqliteTimelineQuery {
        filtered_timeline_query(limit, cursor, SqlEventFilter::default())
    }

    fn filtered_timeline_query(
        limit: usize,
        cursor: Option<SqliteTimelineCursor>,
        filter: SqlEventFilter,
    ) -> SqliteTimelineQuery {
        SqliteTimelineQuery {
            limit: NonZeroUsize::new(limit).expect("test limit should be non-zero"),
            cursor,
            filter,
        }
    }

    fn query_page(store: &SqliteEventStore, query: SqliteTimelineQuery) -> SqliteTimelinePage {
        store
            .query_timeline(query)
            .expect("test timeline query should succeed")
    }

    fn row_ids(events: &[SqliteEventRow]) -> Vec<&str> {
        events.iter().map(|event| event.id.as_str()).collect()
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
