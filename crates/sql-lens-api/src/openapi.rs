#![allow(dead_code)]

use utoipa::OpenApi;

use crate::{
    ApiErrorEnvelope, ConnectionListResponse, ConnectionResponse, HealthResponse,
    ProtocolListResponse, ReplayExecuteRequest, ReplayExecuteResponse, ReplayExecutionResult,
    ReplayPreviewRequest, ReplayPreviewResponse, SqlEventDetailResponse, SqlEventListResponse,
    StatisticsResponse,
};

#[derive(OpenApi)]
#[openapi(
    info(
        title = "SQL Lens API",
        version = "1.0.0",
        description = "REST API for SQL Lens local SQL debugging state."
    ),
    paths(
        health,
        list_sql_events,
        get_sql_event_detail,
        export_sql_events,
        list_connections,
        get_connection_detail,
        get_statistics,
        list_protocols,
        preview_replay,
        execute_replay,
    ),
    components(schemas(
        ApiErrorEnvelope,
        ConnectionListResponse,
        ConnectionResponse,
        HealthResponse,
        ProtocolListResponse,
        ReplayPreviewRequest,
        ReplayPreviewResponse,
        ReplayExecuteRequest,
        ReplayExecuteResponse,
        ReplayExecutionResult,
        SqlEventDetailResponse,
        SqlEventListResponse,
        StatisticsResponse,
    )),
    tags(
        (name = "health", description = "Process health"),
        (name = "sql-events", description = "Captured SQL event timeline"),
        (name = "connections", description = "Observed database connections"),
        (name = "statistics", description = "Live aggregate statistics"),
        (name = "protocols", description = "Supported and planned protocols"),
        (name = "replay", description = "Replay preview helpers")
    )
)]
struct SqlLensOpenApi;

pub fn openapi() -> utoipa::openapi::OpenApi {
    SqlLensOpenApi::openapi()
}

pub fn openapi_yaml() -> Result<String, Box<dyn std::error::Error + Send + Sync + 'static>> {
    Ok(openapi().to_yaml()?)
}

#[utoipa::path(
    get,
    path = "/api/v1/health",
    tag = "health",
    responses(
        (status = 200, description = "Health snapshot", body = HealthResponse),
        (status = 404, description = "Route not found", body = ApiErrorEnvelope)
    )
)]
fn health() {}

#[utoipa::path(
    get,
    path = "/api/v1/sql-events",
    tag = "sql-events",
    params(
        ("limit" = Option<usize>, Query, description = "Maximum number of events to return"),
        ("cursor" = Option<String>, Query, description = "Timeline cursor from a previous response"),
        ("target_name" = Option<String>, Query, description = "Configured proxy target name"),
        ("protocol" = Option<String>, Query, description = "Protocol name"),
        ("database_type" = Option<String>, Query, description = "Database family"),
        ("database" = Option<String>, Query, description = "Database/schema name"),
        ("user" = Option<String>, Query, description = "Database user"),
        ("client_addr" = Option<String>, Query, description = "Client socket address"),
        ("status" = Option<String>, Query, description = "Capture status"),
        ("min_duration_ms" = Option<u64>, Query, description = "Inclusive minimum duration"),
        ("max_duration_ms" = Option<u64>, Query, description = "Inclusive maximum duration"),
        ("q" = Option<String>, Query, description = "SQL text search"),
        ("fingerprint" = Option<String>, Query, description = "SQL fingerprint"),
        ("from" = Option<String>, Query, description = "Inclusive start timestamp"),
        ("to" = Option<String>, Query, description = "Inclusive end timestamp")
    ),
    responses(
        (status = 200, description = "SQL event timeline page", body = SqlEventListResponse),
        (status = 400, description = "Invalid query parameter", body = ApiErrorEnvelope),
        (status = 404, description = "Route not found", body = ApiErrorEnvelope)
    )
)]
fn list_sql_events() {}

#[utoipa::path(
    get,
    path = "/api/v1/sql-events/{id}",
    tag = "sql-events",
    params(("id" = String, Path, description = "SQL event id")),
    responses(
        (status = 200, description = "SQL event detail", body = SqlEventDetailResponse),
        (status = 404, description = "SQL event not found", body = ApiErrorEnvelope)
    )
)]
fn get_sql_event_detail() {}

#[utoipa::path(
    get,
    path = "/api/v1/sql-events/export",
    tag = "sql-events",
    params(
        ("format" = Option<String>, Query, description = "Export format: json or ndjson"),
        ("limit" = Option<usize>, Query, description = "Maximum number of events to export"),
        ("target_name" = Option<String>, Query, description = "Configured proxy target name"),
        ("protocol" = Option<String>, Query, description = "Protocol name"),
        ("database_type" = Option<String>, Query, description = "Database family"),
        ("database" = Option<String>, Query, description = "Database/schema name"),
        ("user" = Option<String>, Query, description = "Database user"),
        ("client_addr" = Option<String>, Query, description = "Client socket address"),
        ("status" = Option<String>, Query, description = "Capture status"),
        ("min_duration_ms" = Option<u64>, Query, description = "Inclusive minimum duration"),
        ("max_duration_ms" = Option<u64>, Query, description = "Inclusive maximum duration"),
        ("q" = Option<String>, Query, description = "SQL text search"),
        ("fingerprint" = Option<String>, Query, description = "SQL fingerprint"),
        ("from" = Option<String>, Query, description = "Inclusive start timestamp"),
        ("to" = Option<String>, Query, description = "Inclusive end timestamp")
    ),
    responses(
        (status = 200, description = "Redacted SQL event export", body = Vec<SqlEventDetailResponse>),
        (status = 400, description = "Invalid export parameter", body = ApiErrorEnvelope)
    )
)]
fn export_sql_events() {}

#[utoipa::path(
    get,
    path = "/api/v1/connections",
    tag = "connections",
    params(("limit" = Option<usize>, Query, description = "Maximum number of connections to return")),
    responses(
        (status = 200, description = "Connection list", body = ConnectionListResponse),
        (status = 400, description = "Invalid query parameter", body = ApiErrorEnvelope)
    )
)]
fn list_connections() {}

#[utoipa::path(
    get,
    path = "/api/v1/connections/{id}",
    tag = "connections",
    params(("id" = String, Path, description = "Connection id")),
    responses(
        (status = 200, description = "Connection detail", body = ConnectionResponse),
        (status = 404, description = "Connection not found", body = ApiErrorEnvelope)
    )
)]
fn get_connection_detail() {}

#[utoipa::path(
    get,
    path = "/api/v1/statistics",
    tag = "statistics",
    params(("window" = Option<String>, Query, description = "Statistics window: 1m or 60s")),
    responses(
        (status = 200, description = "Live statistics", body = StatisticsResponse),
        (status = 400, description = "Invalid query parameter", body = ApiErrorEnvelope)
    )
)]
fn get_statistics() {}

#[utoipa::path(
    get,
    path = "/api/v1/protocols",
    tag = "protocols",
    responses((status = 200, description = "Protocol list", body = ProtocolListResponse))
)]
fn list_protocols() {}

#[utoipa::path(
    post,
    path = "/api/v1/replay/preview",
    tag = "replay",
    request_body(
        content = ReplayPreviewRequest,
        description = "Exactly one of event_id or sql must be provided",
        content_type = "application/json"
    ),
    responses(
        (status = 200, description = "Replay preview", body = ReplayPreviewResponse),
        (status = 400, description = "Invalid replay preview request", body = ApiErrorEnvelope),
        (status = 404, description = "SQL event not found", body = ApiErrorEnvelope)
    )
)]
fn preview_replay() {}

#[utoipa::path(
    post,
    path = "/api/v1/replay/execute",
    tag = "replay",
    request_body(
        content = ReplayExecuteRequest,
        description = "Requires target_name and exactly one of event_id or sql",
        content_type = "application/json"
    ),
    responses(
        (status = 200, description = "Replay execution result", body = ReplayExecuteResponse),
        (status = 400, description = "Invalid replay execution request", body = ApiErrorEnvelope),
        (status = 404, description = "SQL event or target not found", body = ApiErrorEnvelope),
        (status = 409, description = "Replay policy rejected the request", body = ApiErrorEnvelope),
        (status = 503, description = "Replay executor unavailable or timed out", body = ApiErrorEnvelope)
    )
)]
fn execute_replay() {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        HEALTH_PATH, PROTOCOLS_PATH, REPLAY_PREVIEW_PATH, SQL_EVENT_DETAIL_PATH,
        SQL_EVENTS_EXPORT_PATH, SQL_EVENTS_PATH, STATISTICS_PATH,
    };

    #[test]
    fn openapi_contains_current_rest_paths() {
        let document = openapi();
        let paths = &document.paths.paths;

        for expected_path in [
            HEALTH_PATH,
            SQL_EVENTS_PATH,
            SQL_EVENT_DETAIL_PATH,
            SQL_EVENTS_EXPORT_PATH,
            "/api/v1/connections",
            "/api/v1/connections/{id}",
            STATISTICS_PATH,
            PROTOCOLS_PATH,
            REPLAY_PREVIEW_PATH,
        ] {
            assert!(
                paths.contains_key(expected_path),
                "OpenAPI document is missing {expected_path}"
            );
        }
    }

    #[test]
    fn committed_openapi_yaml_is_current() {
        let expected = include_str!("../../../docs/openapi/sql-lens.v1.yaml");
        let generated = openapi_yaml().expect("OpenAPI YAML should serialize");

        assert_eq!(
            generated, expected,
            "regenerate docs/openapi/sql-lens.v1.yaml with: rtk cargo run -p sql-lens-api --example generate-openapi > docs/openapi/sql-lens.v1.yaml"
        );
    }
}
