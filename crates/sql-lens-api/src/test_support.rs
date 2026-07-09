use sql_lens_core::{
    CaptureStatus, ConnectionId, DatabaseType, DurationMillis, MetadataField, MetadataValue,
    ProtocolMetadata, ProtocolName, QueryTiming, ResultSummary, SqlEvent, SqlEventId, SqlEventKind,
    SqlParameter, SqlParameterValue, Timestamp,
};

pub(crate) fn test_event(id: &str) -> SqlEvent {
    SqlEvent {
        id: SqlEventId(id.to_owned()),
        timestamp: Timestamp("2026-07-07T09:00:00Z".to_owned()),
        target_name: Some("mysql-local".to_owned()),
        protocol: ProtocolName("mysql".to_owned()),
        database_type: DatabaseType("mysql".to_owned()),
        connection_id: ConnectionId("conn_1".to_owned()),
        client_addr: "127.0.0.1:51000".to_owned(),
        backend_addr: "127.0.0.1:3306".to_owned(),
        user: Some("app".to_owned()),
        database: Some("app".to_owned()),
        kind: SqlEventKind::StatementExecute,
        status: CaptureStatus::Ok,
        duration: DurationMillis(3),
        original_sql: "SELECT * FROM users WHERE id = ?".to_owned(),
        normalized_sql: Some("select * from users where id = ?".to_owned()),
        expanded_sql: Some("SELECT * FROM users WHERE id = 42".to_owned()),
        fingerprint: Some("select * from users where id = ?".to_owned()),
        parameters: vec![SqlParameter {
            index: 0,
            name: Some("id".to_owned()),
            value: SqlParameterValue::Integer(42),
            redacted: false,
        }],
        result: Some(ResultSummary {
            affected_rows: Some(0),
            returned_rows: Some(1),
        }),
        error: None,
        timings: QueryTiming {
            started_at: Timestamp("2026-07-07T09:00:00Z".to_owned()),
            ended_at: Some(Timestamp("2026-07-07T09:00:00Z".to_owned())),
            duration: DurationMillis(3),
        },
        metadata: ProtocolMetadata {
            protocol: ProtocolName("mysql".to_owned()),
            fields: vec![
                MetadataField {
                    key: "command".to_owned(),
                    value: MetadataValue::String("COM_STMT_EXECUTE".to_owned()),
                },
                MetadataField {
                    key: "statement_id".to_owned(),
                    value: MetadataValue::Unsigned(12),
                },
            ],
        },
    }
}
