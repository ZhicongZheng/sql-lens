//! Protocol-neutral domain models for SQL Lens.

mod error;
mod event;
mod fingerprint;
mod ids;
mod metadata;
mod redaction;
mod time;

pub use error::{ApiError, ApiErrorCode, ErrorSummary};
pub use event::{
    CaptureStatus, ConnectionInfo, ConnectionState, PreparedStatementInfo, QueryTiming,
    ResultSummary, SqlEvent, SqlEventKind, SqlParameter, SqlParameterValue,
};
pub use fingerprint::fingerprint_sql;
pub use ids::{ConnectionId, RequestId, SqlEventId, StatementId};
pub use metadata::{DatabaseType, MetadataField, MetadataValue, ProtocolMetadata, ProtocolName};
pub use redaction::{
    DEFAULT_REDACTION_MASK, DEFAULT_REDACTION_PARAMETER_NAMES, RedactionPolicy, redact_sql_event,
};
pub use time::{DurationMillis, Timestamp};

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use super::*;

    fn assert_serde<T>()
    where
        T: Serialize + for<'de> Deserialize<'de>,
    {
    }

    #[test]
    fn sql_event_can_be_constructed() {
        let event = SqlEvent {
            id: SqlEventId("evt_01".to_owned()),
            timestamp: Timestamp("2026-07-03T12:00:00Z".to_owned()),
            target_name: Some("mysql-local".to_owned()),
            protocol: ProtocolName("mysql".to_owned()),
            database_type: DatabaseType("mysql".to_owned()),
            connection_id: ConnectionId("conn_01".to_owned()),
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
                started_at: Timestamp("2026-07-03T12:00:00Z".to_owned()),
                ended_at: Some(Timestamp("2026-07-03T12:00:00Z".to_owned())),
                duration: DurationMillis(3),
            },
            metadata: ProtocolMetadata {
                protocol: ProtocolName("mysql".to_owned()),
                fields: vec![MetadataField {
                    key: "command".to_owned(),
                    value: MetadataValue::String("COM_STMT_EXECUTE".to_owned()),
                }],
            },
        };

        assert_eq!(event.status, CaptureStatus::Ok);
        assert_eq!(event.parameters.len(), 1);
    }

    #[test]
    fn connection_info_can_be_constructed() {
        let connection = ConnectionInfo {
            id: ConnectionId("conn_01".to_owned()),
            target_name: Some("mysql-local".to_owned()),
            protocol: ProtocolName("mysql".to_owned()),
            database_type: DatabaseType("mysql".to_owned()),
            client_addr: "127.0.0.1:51000".to_owned(),
            backend_addr: "127.0.0.1:3306".to_owned(),
            user: Some("app".to_owned()),
            database: Some("app".to_owned()),
            state: ConnectionState::Ready,
            connected_at: Timestamp("2026-07-03T12:00:00Z".to_owned()),
            closed_at: None,
            last_activity_at: Some(Timestamp("2026-07-03T12:00:01Z".to_owned())),
            bytes_in: 128,
            bytes_out: 256,
            query_count: 1,
        };

        assert_eq!(connection.state, ConnectionState::Ready);
        assert_eq!(connection.query_count, 1);
    }

    #[test]
    fn prepared_statement_and_parameters_can_be_constructed() {
        let statement = PreparedStatementInfo {
            connection_id: ConnectionId("conn_01".to_owned()),
            statement_id: StatementId("stmt_01".to_owned()),
            protocol: ProtocolName("mysql".to_owned()),
            template_sql: "SELECT * FROM users WHERE id = ?".to_owned(),
            parameter_count: 1,
            created_at: Timestamp("2026-07-03T12:00:00Z".to_owned()),
            closed_at: None,
            metadata: ProtocolMetadata {
                protocol: ProtocolName("mysql".to_owned()),
                fields: vec![MetadataField {
                    key: "statement_id".to_owned(),
                    value: MetadataValue::Unsigned(12),
                }],
            },
        };

        let parameters = [
            SqlParameter {
                index: 0,
                name: Some("id".to_owned()),
                value: SqlParameterValue::Unsigned(42),
                redacted: false,
            },
            SqlParameter {
                index: 1,
                name: Some("password".to_owned()),
                value: SqlParameterValue::String("***".to_owned()),
                redacted: true,
            },
        ];

        assert_eq!(statement.parameter_count, 1);
        assert!(parameters[1].redacted);
    }

    #[test]
    fn api_error_can_be_constructed() {
        let error = ApiError {
            code: ApiErrorCode::BadRequest,
            message: "Invalid duration filter".to_owned(),
            request_id: Some(RequestId("req_01".to_owned())),
            details: vec![MetadataField {
                key: "field".to_owned(),
                value: MetadataValue::String("min_duration_ms".to_owned()),
            }],
        };

        assert_eq!(error.code, ApiErrorCode::BadRequest);
        assert_eq!(error.details.len(), 1);
    }

    #[test]
    fn public_models_support_serde_traits() {
        assert_serde::<SqlEvent>();
        assert_serde::<ConnectionInfo>();
        assert_serde::<PreparedStatementInfo>();
        assert_serde::<SqlParameter>();
        assert_serde::<ProtocolMetadata>();
        assert_serde::<ApiError>();
        assert_serde::<RedactionPolicy>();
    }
}
