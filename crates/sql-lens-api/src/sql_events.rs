use std::{collections::BTreeMap, num::NonZeroUsize};

use axum::{
    Extension, Json, Router,
    extract::Query,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use serde::{Deserialize, Serialize};
use sql_lens_core::{
    ApiErrorCode, CaptureStatus, DatabaseType, DurationMillis, MetadataField, MetadataValue,
    ProtocolMetadata, ProtocolName, SqlEvent, SqlEventKind, Timestamp,
};
use sql_lens_storage::{
    RingBufferTimelineCursor, RingBufferTimelineQuery, SqlEventFilter, SqlEventFilterError,
};

use crate::ApiState;

pub const SQL_EVENTS_PATH: &str = "/api/v1/sql-events";
const DEFAULT_LIMIT: usize = 100;
const MAX_LIMIT: usize = 500;
const CURSOR_PREFIX: &str = "seq_";

pub(crate) fn routes() -> Router {
    Router::new().route(SQL_EVENTS_PATH, get(list_sql_events))
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct SqlEventListQueryParams {
    limit: Option<usize>,
    cursor: Option<String>,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SqlEventListResponse {
    pub items: Vec<SqlEventSummaryResponse>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SqlEventSummaryResponse {
    pub id: String,
    pub timestamp: String,
    pub protocol: String,
    pub database_type: String,
    pub connection_id: String,
    pub client_addr: String,
    pub backend_addr: String,
    pub user: Option<String>,
    pub database: Option<String>,
    pub kind: String,
    pub status: String,
    pub duration_ms: u64,
    pub original_sql: String,
    pub expanded_sql: Option<String>,
    pub fingerprint: Option<String>,
    pub rows: Option<RowsSummaryResponse>,
    pub metadata: ProtocolMetadataResponse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RowsSummaryResponse {
    pub affected: Option<u64>,
    pub returned: Option<u64>,
}

pub type ProtocolMetadataResponse = BTreeMap<String, BTreeMap<String, MetadataValueResponse>>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MetadataValueResponse {
    String(String),
    Integer(i64),
    Unsigned(u64),
    Float(f64),
    Boolean(bool),
}

async fn list_sql_events(
    Extension(state): Extension<ApiState>,
    Query(params): Query<SqlEventListQueryParams>,
) -> Result<Json<SqlEventListResponse>, ApiEndpointError> {
    let query = params.try_into_timeline_query()?;
    let page = {
        let event_store = state.event_store();
        let store = event_store.read().await;
        store.query_timeline(query)?
    };

    Ok(Json(SqlEventListResponse {
        items: page
            .events
            .iter()
            .map(SqlEventSummaryResponse::from)
            .collect(),
        next_cursor: page.next_cursor.map(encode_cursor),
    }))
}

impl SqlEventListQueryParams {
    fn try_into_timeline_query(self) -> Result<RingBufferTimelineQuery, ApiEndpointError> {
        Ok(RingBufferTimelineQuery {
            limit: parse_limit(self.limit)?,
            cursor: self.cursor.as_deref().map(decode_cursor).transpose()?,
            filter: SqlEventFilter {
                protocol: self.protocol.map(ProtocolName),
                database_type: self.database_type.map(DatabaseType),
                database: self.database,
                user: self.user,
                client_addr: self.client_addr,
                status: self.status.as_deref().map(parse_status).transpose()?,
                min_duration: self.min_duration_ms.map(DurationMillis),
                max_duration: self.max_duration_ms.map(DurationMillis),
                text: self.q,
                fingerprint: self.fingerprint,
                from: self.from.map(Timestamp),
                to: self.to.map(Timestamp),
            },
        })
    }
}

fn parse_limit(limit: Option<usize>) -> Result<NonZeroUsize, ApiEndpointError> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT);
    NonZeroUsize::new(limit)
        .ok_or_else(|| ApiEndpointError::bad_request("limit must be greater than zero", "limit"))
}

fn parse_status(status: &str) -> Result<CaptureStatus, ApiEndpointError> {
    match status {
        "ok" => Ok(CaptureStatus::Ok),
        "slow" => Ok(CaptureStatus::Slow),
        "error" => Ok(CaptureStatus::Error),
        "unknown" => Ok(CaptureStatus::Unknown),
        _ => Err(ApiEndpointError::bad_request(
            "status must be one of ok, slow, error, unknown",
            "status",
        )),
    }
}

fn encode_cursor(cursor: RingBufferTimelineCursor) -> String {
    format!("{CURSOR_PREFIX}{}", cursor.before_sequence)
}

fn decode_cursor(cursor: &str) -> Result<RingBufferTimelineCursor, ApiEndpointError> {
    let before_sequence = cursor
        .strip_prefix(CURSOR_PREFIX)
        .and_then(|value| value.parse::<u64>().ok())
        .ok_or_else(|| ApiEndpointError::bad_request("invalid cursor", "cursor"))?;

    Ok(RingBufferTimelineCursor { before_sequence })
}

impl From<&SqlEvent> for SqlEventSummaryResponse {
    fn from(event: &SqlEvent) -> Self {
        Self {
            id: event.id.0.clone(),
            timestamp: event.timestamp.0.clone(),
            protocol: event.protocol.0.clone(),
            database_type: event.database_type.0.clone(),
            connection_id: event.connection_id.0.clone(),
            client_addr: event.client_addr.clone(),
            backend_addr: event.backend_addr.clone(),
            user: event.user.clone(),
            database: event.database.clone(),
            kind: event_kind_name(event.kind).to_owned(),
            status: capture_status_name(event.status).to_owned(),
            duration_ms: event.duration.0,
            original_sql: event.original_sql.clone(),
            expanded_sql: event.expanded_sql.clone(),
            fingerprint: event.fingerprint.clone(),
            rows: event.result.map(|result| RowsSummaryResponse {
                affected: result.affected_rows,
                returned: result.returned_rows,
            }),
            metadata: protocol_metadata_response(&event.metadata),
        }
    }
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

fn protocol_metadata_response(metadata: &ProtocolMetadata) -> ProtocolMetadataResponse {
    let fields = metadata
        .fields
        .iter()
        .map(metadata_field_response)
        .collect::<BTreeMap<_, _>>();

    BTreeMap::from([(metadata.protocol.0.clone(), fields)])
}

fn metadata_field_response(field: &MetadataField) -> (String, MetadataValueResponse) {
    (
        field.key.clone(),
        match &field.value {
            MetadataValue::String(value) => MetadataValueResponse::String(value.clone()),
            MetadataValue::Integer(value) => MetadataValueResponse::Integer(*value),
            MetadataValue::Unsigned(value) => MetadataValueResponse::Unsigned(*value),
            MetadataValue::Float(value) => MetadataValueResponse::Float(*value),
            MetadataValue::Boolean(value) => MetadataValueResponse::Boolean(*value),
        },
    )
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct ApiErrorEnvelope {
    error: ApiErrorBody,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct ApiErrorBody {
    code: String,
    message: String,
    request_id: Option<String>,
    details: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ApiEndpointError {
    status: StatusCode,
    code: ApiErrorCode,
    message: String,
    field: Option<String>,
}

impl ApiEndpointError {
    fn bad_request(message: impl Into<String>, field: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code: ApiErrorCode::BadRequest,
            message: message.into(),
            field: Some(field.into()),
        }
    }
}

impl From<SqlEventFilterError> for ApiEndpointError {
    fn from(error: SqlEventFilterError) -> Self {
        match error {
            SqlEventFilterError::InvalidDurationRange { .. } => {
                Self::bad_request("invalid duration filter", "min_duration_ms")
            }
            SqlEventFilterError::InvalidTimestampRange { .. } => {
                Self::bad_request("invalid timestamp filter", "from")
            }
        }
    }
}

impl IntoResponse for ApiEndpointError {
    fn into_response(self) -> Response {
        let details = self
            .field
            .map(|field| BTreeMap::from([("field".to_owned(), field)]))
            .unwrap_or_default();
        let body = ApiErrorEnvelope {
            error: ApiErrorBody {
                code: api_error_code_name(self.code).to_owned(),
                message: self.message,
                request_id: None,
                details,
            },
        };

        (self.status, Json(body)).into_response()
    }
}

fn api_error_code_name(code: ApiErrorCode) -> &'static str {
    match code {
        ApiErrorCode::BadRequest => "BAD_REQUEST",
        ApiErrorCode::Unauthorized => "UNAUTHORIZED",
        ApiErrorCode::Forbidden => "FORBIDDEN",
        ApiErrorCode::NotFound => "NOT_FOUND",
        ApiErrorCode::Conflict => "CONFLICT",
        ApiErrorCode::RateLimited => "RATE_LIMITED",
        ApiErrorCode::Internal => "INTERNAL",
        ApiErrorCode::StorageUnavailable => "STORAGE_UNAVAILABLE",
        ApiErrorCode::ProxyNotReady => "PROXY_NOT_READY",
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use serde_json::Value;
    use sql_lens_core::{
        ConnectionId, MetadataField, MetadataValue, ProtocolMetadata, QueryTiming, ResultSummary,
        SqlEventId,
    };
    use sql_lens_storage::RingBufferStore;
    use tower::ServiceExt;

    use super::*;
    use crate::{REQUEST_ID_HEADER, router_with_state};

    fn capacity(value: usize) -> NonZeroUsize {
        NonZeroUsize::new(value).expect("test capacity should be non-zero")
    }

    fn test_event(id: &str) -> SqlEvent {
        SqlEvent {
            id: SqlEventId(id.to_owned()),
            timestamp: Timestamp("2026-07-07T09:00:00Z".to_owned()),
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
            parameters: Vec::new(),
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

    fn app_with_events(events: Vec<SqlEvent>) -> Router {
        let mut store = RingBufferStore::new(capacity(10));
        for event in events {
            store.append(event);
        }

        router_with_state(ApiState::new(store))
    }

    async fn get_json(app: Router, uri: &str) -> (StatusCode, Value, bool) {
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
        let has_request_id = response.headers().contains_key(REQUEST_ID_HEADER);
        let body = to_bytes(response.into_body(), 4096)
            .await
            .expect("response body should be readable");
        let json = serde_json::from_slice(&body).expect("response should be JSON");

        (status, json, has_request_id)
    }

    #[tokio::test]
    async fn sql_event_list_returns_empty_page() {
        let (status, json, has_request_id) =
            get_json(app_with_events(Vec::new()), SQL_EVENTS_PATH).await;

        assert_eq!(status, StatusCode::OK);
        assert!(has_request_id);
        assert_eq!(json["items"], Value::Array(Vec::new()));
        assert!(json["next_cursor"].is_null());
    }

    #[tokio::test]
    async fn sql_event_list_returns_documented_summary_shape() {
        let (status, json, has_request_id) =
            get_json(app_with_events(vec![test_event("evt_1")]), SQL_EVENTS_PATH).await;

        assert_eq!(status, StatusCode::OK);
        assert!(has_request_id);
        let item = &json["items"][0];
        assert_eq!(item["id"], "evt_1");
        assert_eq!(item["protocol"], "mysql");
        assert_eq!(item["database_type"], "mysql");
        assert_eq!(item["connection_id"], "conn_1");
        assert_eq!(item["kind"], "statement_execute");
        assert_eq!(item["status"], "ok");
        assert_eq!(item["duration_ms"], 3);
        assert_eq!(item["rows"]["affected"], 0);
        assert_eq!(item["rows"]["returned"], 1);
        assert_eq!(item["metadata"]["mysql"]["command"], "COM_STMT_EXECUTE");
        assert_eq!(item["metadata"]["mysql"]["statement_id"], 12);
    }

    #[tokio::test]
    async fn sql_event_list_maps_query_params_to_storage_filters() {
        let mut matching = test_event("evt_1");
        matching.client_addr = "127.0.0.1:51042".to_owned();
        matching.fingerprint = Some("select filtered".to_owned());
        let mut wrong_client = matching.clone();
        wrong_client.id = SqlEventId("evt_2".to_owned());
        wrong_client.client_addr = "127.0.0.1:51043".to_owned();
        let mut wrong_fingerprint = matching.clone();
        wrong_fingerprint.id = SqlEventId("evt_3".to_owned());
        wrong_fingerprint.fingerprint = Some("select other".to_owned());

        let (status, json, _) = get_json(
            app_with_events(vec![matching, wrong_client, wrong_fingerprint]),
            "/api/v1/sql-events?client_addr=127.0.0.1:51042&fingerprint=select%20filtered",
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            json["items"]
                .as_array()
                .expect("items should be array")
                .len(),
            1
        );
        assert_eq!(json["items"][0]["id"], "evt_1");
    }

    #[tokio::test]
    async fn sql_event_list_pages_with_cursor() {
        let (status, first_page, _) = get_json(
            app_with_events(vec![
                test_event("evt_1"),
                test_event("evt_2"),
                test_event("evt_3"),
            ]),
            "/api/v1/sql-events?limit=2",
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(first_page["items"][0]["id"], "evt_3");
        assert_eq!(first_page["items"][1]["id"], "evt_2");
        let cursor = first_page["next_cursor"]
            .as_str()
            .expect("first page should return cursor");

        let (status, second_page, _) = get_json(
            app_with_events(vec![
                test_event("evt_1"),
                test_event("evt_2"),
                test_event("evt_3"),
            ]),
            &format!("/api/v1/sql-events?limit=2&cursor={cursor}"),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(second_page["items"][0]["id"], "evt_1");
        assert!(second_page["next_cursor"].is_null());
    }

    #[tokio::test]
    async fn sql_event_list_rejects_invalid_cursor() {
        let (status, json, has_request_id) = get_json(
            app_with_events(Vec::new()),
            "/api/v1/sql-events?cursor=nope",
        )
        .await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(has_request_id);
        assert_eq!(json["error"]["code"], "BAD_REQUEST");
        assert_eq!(json["error"]["details"]["field"], "cursor");
    }

    #[tokio::test]
    async fn sql_event_list_rejects_invalid_duration_range() {
        let (status, json, has_request_id) = get_json(
            app_with_events(Vec::new()),
            "/api/v1/sql-events?min_duration_ms=10&max_duration_ms=5",
        )
        .await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(has_request_id);
        assert_eq!(json["error"]["code"], "BAD_REQUEST");
        assert_eq!(json["error"]["details"]["field"], "min_duration_ms");
    }
}
