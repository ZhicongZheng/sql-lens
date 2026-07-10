//! Plugin and exporter contracts for SQL Lens.

use std::{error::Error, fmt};

use sql_lens_core::{ConnectionInfo, ErrorSummary, PreparedStatementInfo, SqlEvent};

/// A failure returned by a plugin hook.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginError {
    HookFailed { message: String },
}

impl PluginError {
    pub fn hook_failed(message: impl Into<String>) -> Self {
        Self::HookFailed {
            message: message.into(),
        }
    }
}

impl fmt::Display for PluginError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HookFailed { message } => write!(formatter, "plugin hook failed: {message}"),
        }
    }
}

impl Error for PluginError {}

pub type PluginResult = Result<(), PluginError>;

/// Observes a connection after it has been established.
pub trait OnConnect {
    fn on_connect(&mut self, connection: &ConnectionInfo) -> PluginResult;
}

/// Observes a captured text-query event.
pub trait OnQuery {
    fn on_query(&mut self, event: &SqlEvent, connection: &ConnectionInfo) -> PluginResult;
}

/// Observes a prepared statement after it has been created.
pub trait OnPrepare {
    fn on_prepare(
        &mut self,
        statement: &PreparedStatementInfo,
        connection: &ConnectionInfo,
    ) -> PluginResult;
}

/// Observes a captured prepared-statement execution event.
pub trait OnExecute {
    fn on_execute(&mut self, event: &SqlEvent, connection: &ConnectionInfo) -> PluginResult;
}

/// Observes a captured SQL event with an error summary.
pub trait OnError {
    fn on_error(
        &mut self,
        event: &SqlEvent,
        connection: &ConnectionInfo,
        error: &ErrorSummary,
    ) -> PluginResult;
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use sql_lens_core::{
        CaptureStatus, ConnectionId, ConnectionState, DatabaseType, DurationMillis, ErrorSummary,
        MetadataField, MetadataValue, PreparedStatementInfo, ProtocolMetadata, ProtocolName,
        QueryTiming, ResultSummary, SqlEvent, SqlEventId, SqlEventKind, SqlParameter,
        SqlParameterValue, StatementId, Timestamp,
    };

    use super::{OnConnect, OnError, OnExecute, OnPrepare, OnQuery, PluginError};

    #[derive(Default)]
    struct RecordingPlugin {
        calls: Vec<&'static str>,
        query_sql: Option<String>,
        prepare_parameter_count: Option<u16>,
        execute_sql: Option<String>,
        error_message: Option<String>,
    }

    impl OnConnect for RecordingPlugin {
        fn on_connect(
            &mut self,
            _connection: &sql_lens_core::ConnectionInfo,
        ) -> super::PluginResult {
            self.calls.push("connect");
            Ok(())
        }
    }

    impl OnQuery for RecordingPlugin {
        fn on_query(
            &mut self,
            event: &SqlEvent,
            _connection: &sql_lens_core::ConnectionInfo,
        ) -> super::PluginResult {
            self.calls.push("query");
            self.query_sql = Some(event.original_sql.clone());
            Ok(())
        }
    }

    impl OnPrepare for RecordingPlugin {
        fn on_prepare(
            &mut self,
            statement: &PreparedStatementInfo,
            _connection: &sql_lens_core::ConnectionInfo,
        ) -> super::PluginResult {
            self.calls.push("prepare");
            self.prepare_parameter_count = Some(statement.parameter_count);
            Ok(())
        }
    }

    impl OnExecute for RecordingPlugin {
        fn on_execute(
            &mut self,
            event: &SqlEvent,
            _connection: &sql_lens_core::ConnectionInfo,
        ) -> super::PluginResult {
            self.calls.push("execute");
            self.execute_sql = event.expanded_sql.clone();
            Ok(())
        }
    }

    impl OnError for RecordingPlugin {
        fn on_error(
            &mut self,
            _event: &SqlEvent,
            _connection: &sql_lens_core::ConnectionInfo,
            error: &ErrorSummary,
        ) -> super::PluginResult {
            self.calls.push("error");
            self.error_message = Some(error.message.clone());
            Ok(())
        }
    }

    struct FailingQueryPlugin;

    impl OnQuery for FailingQueryPlugin {
        fn on_query(
            &mut self,
            _event: &SqlEvent,
            _connection: &sql_lens_core::ConnectionInfo,
        ) -> super::PluginResult {
            Err(PluginError::hook_failed("exporter unavailable"))
        }
    }

    #[test]
    fn hooks_are_object_safe_and_receive_protocol_neutral_payloads() {
        let connection = sample_connection();
        let mut event = sample_event();
        let statement = sample_statement();
        let error = ErrorSummary {
            code: Some("1146".to_owned()),
            sql_state: Some("42S02".to_owned()),
            message: "table does not exist".to_owned(),
            metadata: Some(ProtocolMetadata {
                protocol: ProtocolName("mysql".to_owned()),
                fields: vec![MetadataField {
                    key: "vendor_code".to_owned(),
                    value: MetadataValue::Unsigned(1146),
                }],
            }),
        };
        event.error = Some(error.clone());
        let mut plugin = RecordingPlugin::default();

        {
            let hook: &mut dyn OnConnect = &mut plugin;
            hook.on_connect(&connection).expect("connect hook succeeds");
        }
        {
            let hook: &mut dyn OnQuery = &mut plugin;
            hook.on_query(&event, &connection)
                .expect("query hook succeeds");
        }
        {
            let hook: &mut dyn OnPrepare = &mut plugin;
            hook.on_prepare(&statement, &connection)
                .expect("prepare hook succeeds");
        }
        {
            let hook: &mut dyn OnExecute = &mut plugin;
            hook.on_execute(&event, &connection)
                .expect("execute hook succeeds");
        }
        {
            let hook: &mut dyn OnError = &mut plugin;
            hook.on_error(&event, &connection, &error)
                .expect("error hook succeeds");
        }

        assert_eq!(
            plugin.calls,
            ["connect", "query", "prepare", "execute", "error"]
        );
        assert_eq!(
            plugin.query_sql.as_deref(),
            Some("SELECT * FROM users WHERE id = ?")
        );
        assert_eq!(plugin.prepare_parameter_count, Some(1));
        assert_eq!(
            plugin.execute_sql.as_deref(),
            Some("SELECT * FROM users WHERE id = 42")
        );
        assert_eq!(
            plugin.error_message.as_deref(),
            Some("table does not exist")
        );
    }

    #[test]
    fn hook_failures_are_typed_standard_errors() {
        let connection = sample_connection();
        let event = sample_event();
        let mut hook: Box<dyn OnQuery> = Box::new(FailingQueryPlugin);

        let error = hook
            .on_query(&event, &connection)
            .expect_err("hook failure is returned to the dispatcher");

        assert_eq!(error, PluginError::hook_failed("exporter unavailable"));
        assert_eq!(
            error.to_string(),
            "plugin hook failed: exporter unavailable"
        );
        assert_standard_error(&error);
    }

    fn assert_standard_error(_: &dyn Error) {}

    fn sample_connection() -> sql_lens_core::ConnectionInfo {
        sql_lens_core::ConnectionInfo {
            id: ConnectionId("conn_01".to_owned()),
            target_name: Some("mysql-local".to_owned()),
            protocol: ProtocolName("mysql".to_owned()),
            database_type: DatabaseType("mysql".to_owned()),
            client_addr: "127.0.0.1:51000".to_owned(),
            backend_addr: "127.0.0.1:3306".to_owned(),
            user: Some("app".to_owned()),
            database: Some("app".to_owned()),
            state: ConnectionState::Ready,
            connected_at: Timestamp("2026-07-10T12:00:00Z".to_owned()),
            closed_at: None,
            last_activity_at: Some(Timestamp("2026-07-10T12:00:01Z".to_owned())),
            bytes_in: 128,
            bytes_out: 256,
            query_count: 1,
        }
    }

    fn sample_event() -> SqlEvent {
        SqlEvent {
            id: SqlEventId("evt_01".to_owned()),
            timestamp: Timestamp("2026-07-10T12:00:00Z".to_owned()),
            target_name: Some("mysql-local".to_owned()),
            protocol: ProtocolName("mysql".to_owned()),
            database_type: DatabaseType("mysql".to_owned()),
            connection_id: ConnectionId("conn_01".to_owned()),
            client_addr: "127.0.0.1:51000".to_owned(),
            backend_addr: "127.0.0.1:3306".to_owned(),
            user: Some("app".to_owned()),
            database: Some("app".to_owned()),
            kind: SqlEventKind::StatementExecute,
            status: CaptureStatus::Error,
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
                started_at: Timestamp("2026-07-10T12:00:00Z".to_owned()),
                ended_at: Some(Timestamp("2026-07-10T12:00:00Z".to_owned())),
                duration: DurationMillis(3),
            },
            metadata: ProtocolMetadata {
                protocol: ProtocolName("mysql".to_owned()),
                fields: vec![MetadataField {
                    key: "command".to_owned(),
                    value: MetadataValue::String("COM_STMT_EXECUTE".to_owned()),
                }],
            },
        }
    }

    fn sample_statement() -> PreparedStatementInfo {
        PreparedStatementInfo {
            connection_id: ConnectionId("conn_01".to_owned()),
            statement_id: StatementId("stmt_01".to_owned()),
            protocol: ProtocolName("mysql".to_owned()),
            template_sql: "SELECT * FROM users WHERE id = ?".to_owned(),
            parameter_count: 1,
            created_at: Timestamp("2026-07-10T12:00:00Z".to_owned()),
            closed_at: None,
            metadata: ProtocolMetadata {
                protocol: ProtocolName("mysql".to_owned()),
                fields: vec![],
            },
        }
    }
}
