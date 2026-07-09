use std::num::NonZeroUsize;

use axum::{
    Extension, Router,
    body::Body,
    extract::Query,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use serde::Deserialize;
use sql_lens_core::{RedactionPolicy, redact_sql_event};
use sql_lens_storage::RingBufferTimelineQuery;

use crate::{
    ApiState,
    api_error::ApiEndpointError,
    sql_events::{SqlEventDetailResponse, SqlEventFilterQueryParams},
};

pub const SQL_EVENTS_EXPORT_PATH: &str = "/api/v1/sql-events/export";
pub const MAX_EXPORT_LIMIT: usize = 10_000;

pub(crate) fn routes() -> Router {
    Router::new().route(SQL_EVENTS_EXPORT_PATH, get(export_sql_events))
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
struct SqlEventExportQueryParams {
    format: Option<String>,
    limit: Option<usize>,
    target_name: Option<String>,
    protocol: Option<String>,
    database_type: Option<String>,
    database: Option<String>,
    user: Option<String>,
    client_addr: Option<String>,
    status: Option<String>,
    min_duration_ms: Option<u64>,
    max_duration_ms: Option<u64>,
    q: Option<String>,
    fingerprint: Option<String>,
    from: Option<String>,
    to: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExportFormat {
    Json,
    Ndjson,
}

async fn export_sql_events(
    Extension(state): Extension<ApiState>,
    Query(params): Query<SqlEventExportQueryParams>,
) -> Result<Response, ApiEndpointError> {
    let format = parse_export_format(params.format.as_deref())?;
    let query = RingBufferTimelineQuery {
        limit: parse_export_limit(params.limit)?,
        cursor: None,
        filter: params.into_filter_params().try_into_filter()?,
    };
    let events = {
        let event_store = state.event_store();
        let store = event_store.read().await;
        store.query_timeline(query)?
    }
    .events
    .into_iter()
    .map(|event| redact_sql_event(event, &RedactionPolicy::default()))
    .map(|event| SqlEventDetailResponse::from(&event))
    .collect::<Vec<_>>();

    match format {
        ExportFormat::Json => Ok(axum::Json(events).into_response()),
        ExportFormat::Ndjson => ndjson_response(&events),
    }
}

impl SqlEventExportQueryParams {
    fn into_filter_params(self) -> SqlEventFilterQueryParams {
        SqlEventFilterQueryParams {
            target_name: self.target_name,
            protocol: self.protocol,
            database_type: self.database_type,
            database: self.database,
            user: self.user,
            client_addr: self.client_addr,
            status: self.status,
            min_duration_ms: self.min_duration_ms,
            max_duration_ms: self.max_duration_ms,
            q: self.q,
            fingerprint: self.fingerprint,
            from: self.from,
            to: self.to,
        }
    }
}

fn parse_export_format(format: Option<&str>) -> Result<ExportFormat, ApiEndpointError> {
    match format {
        None | Some("json") => Ok(ExportFormat::Json),
        Some("ndjson") => Ok(ExportFormat::Ndjson),
        Some(_) => Err(ApiEndpointError::bad_request(
            "format must be one of json, ndjson",
            "format",
        )),
    }
}

fn parse_export_limit(limit: Option<usize>) -> Result<NonZeroUsize, ApiEndpointError> {
    let limit = limit.unwrap_or(MAX_EXPORT_LIMIT).min(MAX_EXPORT_LIMIT);
    NonZeroUsize::new(limit)
        .ok_or_else(|| ApiEndpointError::bad_request("limit must be greater than zero", "limit"))
}

fn ndjson_response(events: &[SqlEventDetailResponse]) -> Result<Response, ApiEndpointError> {
    let mut body = String::new();

    for event in events {
        let line = serde_json::to_string(event).map_err(|_| {
            ApiEndpointError::bad_request("failed to serialize export event", "format")
        })?;
        body.push_str(&line);
        body.push('\n');
    }

    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/x-ndjson")],
        Body::from(body),
    )
        .into_response())
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use axum::{
        body::to_bytes,
        http::{Request, StatusCode, header},
    };
    use serde_json::Value;
    use sql_lens_core::{CaptureStatus, SqlEvent, SqlParameterValue};
    use sql_lens_storage::RingBufferStore;
    use tower::ServiceExt;

    use super::*;
    use crate::{REQUEST_ID_HEADER, router_with_state, test_support::test_event};

    fn capacity(value: usize) -> NonZeroUsize {
        NonZeroUsize::new(value).expect("test capacity should be non-zero")
    }

    fn app_with_events(events: Vec<SqlEvent>) -> Router {
        let mut store = RingBufferStore::new(capacity(20));
        for event in events {
            store.append(event);
        }

        router_with_state(ApiState::new(store))
    }

    async fn get(app: Router, uri: &str) -> (StatusCode, String, Option<String>, bool) {
        let response = app
            .oneshot(
                Request::builder()
                    .uri(uri)
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should be handled");
        let status = response.status();
        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let has_request_id = response.headers().contains_key(REQUEST_ID_HEADER);
        let body = to_bytes(response.into_body(), 1024 * 1024)
            .await
            .expect("response body should be readable");
        let body = String::from_utf8(body.to_vec()).expect("response should be UTF-8");

        (status, body, content_type, has_request_id)
    }

    #[tokio::test]
    async fn export_returns_filtered_events_as_json_by_default() {
        let mut matching = test_event("evt_1");
        matching.status = CaptureStatus::Error;
        let other = test_event("evt_2");

        let (status, body, content_type, has_request_id) = get(
            app_with_events(vec![matching, other]),
            "/api/v1/sql-events/export?status=error",
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert!(has_request_id);
        assert!(
            content_type
                .as_deref()
                .is_some_and(|value| value.starts_with("application/json"))
        );
        let json: Value = serde_json::from_str(&body).expect("export should be JSON");
        assert_eq!(
            json.as_array()
                .expect("JSON export should be an array")
                .len(),
            1
        );
        assert_eq!(json[0]["id"], "evt_1");
        assert_eq!(json[0]["status"], "error");
    }

    #[tokio::test]
    async fn export_returns_filtered_events_as_ndjson() {
        let mut matching = test_event("evt_1");
        matching.database = Some("analytics".to_owned());
        let other = test_event("evt_2");

        let (status, body, content_type, has_request_id) = get(
            app_with_events(vec![matching, other]),
            "/api/v1/sql-events/export?format=ndjson&database=analytics",
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert!(has_request_id);
        assert_eq!(content_type.as_deref(), Some("application/x-ndjson"));
        let lines = body.lines().collect::<Vec<_>>();
        assert_eq!(lines.len(), 1);
        let event: Value = serde_json::from_str(lines[0]).expect("NDJSON line should be JSON");
        assert_eq!(event["id"], "evt_1");
        assert_eq!(event["database"], "analytics");
    }

    #[tokio::test]
    async fn export_redacts_sensitive_parameters_and_sql_text() {
        let mut event = test_event("evt_1");
        event.original_sql = "UPDATE users SET password = ? WHERE id = ?".to_owned();
        event.expanded_sql = Some("UPDATE users SET password = 's3cr3t' WHERE id = 42".to_owned());
        event.parameters[0].name = Some("password".to_owned());
        event.parameters[0].value = SqlParameterValue::String("s3cr3t".to_owned());

        let (status, body, _, _) = get(app_with_events(vec![event]), SQL_EVENTS_EXPORT_PATH).await;

        assert_eq!(status, StatusCode::OK);
        let json: Value = serde_json::from_str(&body).expect("export should be JSON");
        assert_eq!(
            json[0]["expanded_sql"],
            "UPDATE users SET password = '***' WHERE id = 42"
        );
        assert_eq!(json[0]["parameters"][0]["redacted"], true);
        assert_eq!(json[0]["parameters"][0]["value"]["value"], "***");
    }

    #[tokio::test]
    async fn export_limit_bounds_results() {
        let (status, body, _, _) = get(
            app_with_events(vec![test_event("evt_1"), test_event("evt_2")]),
            "/api/v1/sql-events/export?limit=1",
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        let json: Value = serde_json::from_str(&body).expect("export should be JSON");
        assert_eq!(
            json.as_array()
                .expect("JSON export should be an array")
                .len(),
            1
        );
        assert_eq!(json[0]["id"], "evt_2");
    }

    #[tokio::test]
    async fn export_rejects_invalid_filter() {
        let (status, body, _, has_request_id) = get(
            app_with_events(vec![test_event("evt_1")]),
            "/api/v1/sql-events/export?status=bogus",
        )
        .await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(has_request_id);
        let json: Value = serde_json::from_str(&body).expect("error should be JSON");
        assert_eq!(json["error"]["code"], "BAD_REQUEST");
        assert_eq!(json["error"]["details"]["field"], "status");
    }

    #[tokio::test]
    async fn export_rejects_invalid_format() {
        let (status, body, _, has_request_id) = get(
            app_with_events(vec![test_event("evt_1")]),
            "/api/v1/sql-events/export?format=csv",
        )
        .await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(has_request_id);
        let json: Value = serde_json::from_str(&body).expect("error should be JSON");
        assert_eq!(json["error"]["code"], "BAD_REQUEST");
        assert_eq!(json["error"]["details"]["field"], "format");
    }
}
