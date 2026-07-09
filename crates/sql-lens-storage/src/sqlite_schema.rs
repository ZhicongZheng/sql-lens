use rusqlite::Connection;

pub const SQLITE_SCHEMA_VERSION: i64 = 1;

const SQLITE_SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS schema_version (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE IF NOT EXISTS sql_events (
    id TEXT PRIMARY KEY,
    timestamp TEXT NOT NULL,
    target_name TEXT,
    protocol TEXT NOT NULL,
    database_type TEXT NOT NULL,
    connection_id TEXT NOT NULL,
    client_addr TEXT NOT NULL,
    backend_addr TEXT NOT NULL,
    user_name TEXT,
    database_name TEXT,
    kind TEXT NOT NULL,
    status TEXT NOT NULL,
    duration_ms INTEGER NOT NULL,
    original_sql TEXT NOT NULL,
    normalized_sql TEXT,
    expanded_sql TEXT,
    fingerprint TEXT,
    affected_rows INTEGER,
    returned_rows INTEGER,
    error_code TEXT,
    error_sql_state TEXT,
    error_message TEXT,
    started_at TEXT NOT NULL,
    ended_at TEXT,
    metadata_json TEXT NOT NULL DEFAULT '{}'
);

CREATE TABLE IF NOT EXISTS sql_parameters (
    event_id TEXT NOT NULL,
    parameter_index INTEGER NOT NULL,
    name TEXT,
    value_type TEXT NOT NULL,
    value_json TEXT,
    redacted INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (event_id, parameter_index),
    FOREIGN KEY (event_id) REFERENCES sql_events(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS connections (
    id TEXT PRIMARY KEY,
    target_name TEXT,
    protocol TEXT NOT NULL,
    database_type TEXT NOT NULL,
    client_addr TEXT NOT NULL,
    backend_addr TEXT NOT NULL,
    user_name TEXT,
    database_name TEXT,
    state TEXT NOT NULL,
    connected_at TEXT NOT NULL,
    closed_at TEXT,
    last_activity_at TEXT,
    bytes_in INTEGER NOT NULL DEFAULT 0,
    bytes_out INTEGER NOT NULL DEFAULT 0,
    query_count INTEGER NOT NULL DEFAULT 0,
    metadata_json TEXT NOT NULL DEFAULT '{}'
);

CREATE TABLE IF NOT EXISTS prepared_statements (
    connection_id TEXT NOT NULL,
    statement_key TEXT NOT NULL,
    protocol TEXT NOT NULL,
    template_sql TEXT NOT NULL,
    parameter_count INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    closed_at TEXT,
    metadata_json TEXT NOT NULL DEFAULT '{}',
    PRIMARY KEY (connection_id, statement_key)
);

CREATE INDEX IF NOT EXISTS idx_sql_events_timestamp ON sql_events(timestamp);
CREATE INDEX IF NOT EXISTS idx_sql_events_protocol_timestamp ON sql_events(protocol, timestamp);
CREATE INDEX IF NOT EXISTS idx_sql_events_database_type_timestamp ON sql_events(database_type, timestamp);
CREATE INDEX IF NOT EXISTS idx_sql_events_database_timestamp ON sql_events(database_name, timestamp);
CREATE INDEX IF NOT EXISTS idx_sql_events_user_timestamp ON sql_events(user_name, timestamp);
CREATE INDEX IF NOT EXISTS idx_sql_events_status_timestamp ON sql_events(status, timestamp);
CREATE INDEX IF NOT EXISTS idx_sql_events_fingerprint_timestamp ON sql_events(fingerprint, timestamp);
CREATE INDEX IF NOT EXISTS idx_sql_events_duration_ms ON sql_events(duration_ms);

INSERT OR IGNORE INTO schema_version (version) VALUES (1);
"#;

pub fn apply_sqlite_schema(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute_batch(SQLITE_SCHEMA_SQL)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqlite_schema_applies_to_empty_database() {
        let connection = Connection::open_in_memory().expect("in-memory database should open");

        apply_sqlite_schema(&connection).expect("schema should apply");

        for table in [
            "schema_version",
            "sql_events",
            "sql_parameters",
            "connections",
            "prepared_statements",
        ] {
            assert!(table_exists(&connection, table), "missing table {table}");
        }
    }

    #[test]
    fn sqlite_schema_records_version() {
        let connection = Connection::open_in_memory().expect("in-memory database should open");

        apply_sqlite_schema(&connection).expect("schema should apply");

        let version: i64 = connection
            .query_row("SELECT version FROM schema_version", [], |row| row.get(0))
            .expect("schema version row should exist");

        assert_eq!(version, SQLITE_SCHEMA_VERSION);
    }

    #[test]
    fn sqlite_schema_creates_recommended_indexes() {
        let connection = Connection::open_in_memory().expect("in-memory database should open");

        apply_sqlite_schema(&connection).expect("schema should apply");

        for index in [
            "idx_sql_events_timestamp",
            "idx_sql_events_protocol_timestamp",
            "idx_sql_events_database_type_timestamp",
            "idx_sql_events_database_timestamp",
            "idx_sql_events_user_timestamp",
            "idx_sql_events_status_timestamp",
            "idx_sql_events_fingerprint_timestamp",
            "idx_sql_events_duration_ms",
        ] {
            assert!(index_exists(&connection, index), "missing index {index}");
        }
    }

    #[test]
    fn sqlite_schema_is_idempotent() {
        let connection = Connection::open_in_memory().expect("in-memory database should open");

        apply_sqlite_schema(&connection).expect("schema should apply");
        apply_sqlite_schema(&connection).expect("schema should apply twice");

        let version_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM schema_version", [], |row| row.get(0))
            .expect("schema version count should be readable");

        assert_eq!(version_count, 1);
    }

    #[test]
    fn sqlite_schema_tables_include_storage_contract_columns() {
        let connection = Connection::open_in_memory().expect("in-memory database should open");

        apply_sqlite_schema(&connection).expect("schema should apply");

        for column in [
            "id",
            "timestamp",
            "target_name",
            "protocol",
            "database_type",
            "connection_id",
            "client_addr",
            "backend_addr",
            "user_name",
            "database_name",
            "kind",
            "status",
            "duration_ms",
            "original_sql",
            "normalized_sql",
            "expanded_sql",
            "fingerprint",
            "started_at",
            "ended_at",
            "metadata_json",
        ] {
            assert!(
                table_column_exists(&connection, "sql_events", column),
                "missing sql_events column {column}"
            );
        }
    }

    fn table_exists(connection: &Connection, table: &str) -> bool {
        object_exists(connection, "table", table)
    }

    fn index_exists(connection: &Connection, index: &str) -> bool {
        object_exists(connection, "index", index)
    }

    fn object_exists(connection: &Connection, object_type: &str, name: &str) -> bool {
        connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = ?1 AND name = ?2",
                [object_type, name],
                |row| row.get::<_, i64>(0),
            )
            .expect("sqlite_master query should run")
            == 1
    }

    fn table_column_exists(connection: &Connection, table: &str, column: &str) -> bool {
        let mut statement = connection
            .prepare(&format!("PRAGMA table_info({table})"))
            .expect("table info statement should prepare");
        let columns = statement
            .query_map([], |row| row.get::<_, String>(1))
            .expect("table info should query")
            .collect::<Result<Vec<_>, _>>()
            .expect("table info rows should read");

        columns.iter().any(|candidate| candidate == column)
    }
}
