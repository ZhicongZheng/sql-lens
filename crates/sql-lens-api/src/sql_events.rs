use std::{collections::BTreeMap, num::NonZeroUsize};

use axum::{
    Extension, Json, Router,
    extract::{Path, Query},
    routing::get,
};
use serde::{Deserialize, Serialize};
use sql_lens_core::{
    CaptureStatus, DatabaseType, DurationMillis, MetadataField, MetadataValue, ProtocolMetadata,
    ProtocolName, SqlEvent, SqlEventId, SqlEventKind, SqlParameter, SqlParameterValue, Timestamp,
};
use sql_lens_storage::{
    RingBufferTimelineCursor, RingBufferTimelineQuery, SqlEventFilter, SqlEventFilterError,
};

use crate::{ApiState, api_error::ApiEndpointError};

pub const SQL_EVENTS_PATH: &str = "/api/v1/sql-events";
pub const SQL_EVENT_DETAIL_PATH: &str = "/api/v1/sql-events/{id}";
const DEFAULT_LIMIT: usize = 100;
const MAX_LIMIT: usize = 500;
const CURSOR_PREFIX: &str = "seq_";

pub(crate) fn routes() -> Router {
    Router::new()
        .route(SQL_EVENTS_PATH, get(list_sql_events))
        .route(SQL_EVENT_DETAIL_PATH, get(get_sql_event_detail))
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct SqlEventListQueryParams {
    limit: Option<usize>,
    cursor: Option<String>,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SqlEventListResponse {
    pub items: Vec<SqlEventSummaryResponse>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SqlEventSummaryResponse {
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
    pub duration_ms: u64,
    pub original_sql: String,
    pub expanded_sql: Option<String>,
    pub fingerprint: Option<String>,
    pub rows: Option<RowsSummaryResponse>,
    pub metadata: ProtocolMetadataResponse,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SqlEventDetailResponse {
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
    pub duration_ms: u64,
    pub original_sql: String,
    pub normalized_sql: Option<String>,
    pub expanded_sql: Option<String>,
    pub fingerprint: Option<String>,
    pub parameters: Vec<SqlParameterResponse>,
    pub timings: QueryTimingResponse,
    pub rows: Option<RowsSummaryResponse>,
    pub error: Option<ErrorSummaryResponse>,
    pub metadata: ProtocolMetadataResponse,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SqlParameterResponse {
    pub index: u16,
    pub name: Option<String>,
    pub value: SqlParameterValueResponse,
    pub redacted: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SqlParameterValueResponse {
    #[serde(rename = "type")]
    pub value_type: String,
    pub value: Option<SqlParameterValueDataResponse>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SqlParameterValueDataResponse {
    Integer(i64),
    Unsigned(u64),
    Float(f64),
    Boolean(bool),
    String(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryTimingResponse {
    pub started_at: String,
    pub ended_at: Option<String>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ErrorSummaryResponse {
    pub code: Option<String>,
    pub sql_state: Option<String>,
    pub message: String,
    pub metadata: Option<ProtocolMetadataResponse>,
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

async fn get_sql_event_detail(
    Extension(state): Extension<ApiState>,
    Path(id): Path<String>,
) -> Result<Json<SqlEventDetailResponse>, ApiEndpointError> {
    let event = {
        let event_store = state.event_store();
        let store = event_store.read().await;
        store.get(&SqlEventId(id.clone())).cloned()
    }
    .ok_or_else(|| ApiEndpointError::not_found("SQL event not found", "id", id))?;

    Ok(Json(SqlEventDetailResponse::from(&event)))
}

impl SqlEventListQueryParams {
    fn try_into_timeline_query(self) -> Result<RingBufferTimelineQuery, ApiEndpointError> {
        Ok(RingBufferTimelineQuery {
            limit: parse_limit(self.limit)?,
            cursor: self.cursor.as_deref().map(decode_cursor).transpose()?,
            filter: SqlEventFilter {
                target_name: self.target_name,
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
            id: event_id(event),
            timestamp: timestamp_value(&event.timestamp),
            target_name: event.target_name.clone(),
            protocol: protocol_name(&event.protocol),
            database_type: database_type_name(&event.database_type),
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
            rows: rows_summary(event),
            metadata: protocol_metadata_response(&event.metadata),
        }
    }
}

impl From<&SqlEvent> for SqlEventDetailResponse {
    fn from(event: &SqlEvent) -> Self {
        Self {
            id: event_id(event),
            timestamp: timestamp_value(&event.timestamp),
            target_name: event.target_name.clone(),
            protocol: protocol_name(&event.protocol),
            database_type: database_type_name(&event.database_type),
            connection_id: event.connection_id.0.clone(),
            client_addr: event.client_addr.clone(),
            backend_addr: event.backend_addr.clone(),
            user: event.user.clone(),
            database: event.database.clone(),
            kind: event_kind_name(event.kind).to_owned(),
            status: capture_status_name(event.status).to_owned(),
            duration_ms: event.duration.0,
            original_sql: event.original_sql.clone(),
            normalized_sql: event.normalized_sql.clone(),
            expanded_sql: event.expanded_sql.clone(),
            fingerprint: event.fingerprint.clone(),
            parameters: event
                .parameters
                .iter()
                .map(SqlParameterResponse::from)
                .collect(),
            timings: QueryTimingResponse {
                started_at: timestamp_value(&event.timings.started_at),
                ended_at: event.timings.ended_at.as_ref().map(timestamp_value),
                duration_ms: event.timings.duration.0,
            },
            rows: rows_summary(event),
            error: event.error.as_ref().map(|error| ErrorSummaryResponse {
                code: error.code.clone(),
                sql_state: error.sql_state.clone(),
                message: error.message.clone(),
                metadata: error.metadata.as_ref().map(protocol_metadata_response),
            }),
            metadata: protocol_metadata_response(&event.metadata),
        }
    }
}

impl From<&SqlParameter> for SqlParameterResponse {
    fn from(parameter: &SqlParameter) -> Self {
        Self {
            index: parameter.index,
            name: parameter.name.clone(),
            value: SqlParameterValueResponse::from(&parameter.value),
            redacted: parameter.redacted,
        }
    }
}

impl From<&SqlParameterValue> for SqlParameterValueResponse {
    fn from(value: &SqlParameterValue) -> Self {
        match value {
            SqlParameterValue::Null => Self::new("null", None),
            SqlParameterValue::Integer(value) => Self::new(
                "integer",
                Some(SqlParameterValueDataResponse::Integer(*value)),
            ),
            SqlParameterValue::Unsigned(value) => Self::new(
                "unsigned",
                Some(SqlParameterValueDataResponse::Unsigned(*value)),
            ),
            SqlParameterValue::Float(value) => {
                Self::new("float", Some(SqlParameterValueDataResponse::Float(*value)))
            }
            SqlParameterValue::Boolean(value) => Self::new(
                "boolean",
                Some(SqlParameterValueDataResponse::Boolean(*value)),
            ),
            SqlParameterValue::String(value) => Self::new(
                "string",
                Some(SqlParameterValueDataResponse::String(value.clone())),
            ),
            SqlParameterValue::Date(value) => Self::new(
                "date",
                Some(SqlParameterValueDataResponse::String(value.clone())),
            ),
            SqlParameterValue::Time(value) => Self::new(
                "time",
                Some(SqlParameterValueDataResponse::String(value.clone())),
            ),
            SqlParameterValue::Timestamp(value) => Self::new(
                "timestamp",
                Some(SqlParameterValueDataResponse::String(value.clone())),
            ),
            SqlParameterValue::Json(value) => Self::new(
                "json",
                Some(SqlParameterValueDataResponse::String(value.clone())),
            ),
            SqlParameterValue::BinarySummary(value) => Self::new(
                "binary_summary",
                Some(SqlParameterValueDataResponse::String(value.clone())),
            ),
            SqlParameterValue::Unsupported(value) => Self::new(
                "unsupported",
                Some(SqlParameterValueDataResponse::String(value.clone())),
            ),
        }
    }
}

impl SqlParameterValueResponse {
    fn new(value_type: impl Into<String>, value: Option<SqlParameterValueDataResponse>) -> Self {
        Self {
            value_type: value_type.into(),
            value,
        }
    }
}

fn event_id(event: &SqlEvent) -> String {
    event.id.0.clone()
}

fn timestamp_value(timestamp: &Timestamp) -> String {
    timestamp.0.clone()
}

fn protocol_name(protocol: &ProtocolName) -> String {
    protocol.0.clone()
}

fn database_type_name(database_type: &DatabaseType) -> String {
    database_type.0.clone()
}

fn rows_summary(event: &SqlEvent) -> Option<RowsSummaryResponse> {
    event.result.map(|result| RowsSummaryResponse {
        affected: result.affected_rows,
        returned: result.returned_rows,
    })
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

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use serde_json::Value;
    use sql_lens_core::{
        ConnectionId, ErrorSummary, MetadataField, MetadataValue, ProtocolMetadata, QueryTiming,
        ResultSummary, SqlEventId,
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
        assert_eq!(item["target_name"], "mysql-local");
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
        matching.target_name = Some("mysql-local".to_owned());
        matching.fingerprint = Some("select filtered".to_owned());
        let mut wrong_client = matching.clone();
        wrong_client.id = SqlEventId("evt_2".to_owned());
        wrong_client.client_addr = "127.0.0.1:51043".to_owned();
        let mut wrong_fingerprint = matching.clone();
        wrong_fingerprint.id = SqlEventId("evt_3".to_owned());
        wrong_fingerprint.fingerprint = Some("select other".to_owned());
        let mut wrong_target = matching.clone();
        wrong_target.id = SqlEventId("evt_4".to_owned());
        wrong_target.target_name = Some("starrocks-local".to_owned());

        let (status, json, _) = get_json(
            app_with_events(vec![matching, wrong_client, wrong_fingerprint, wrong_target]),
            "/api/v1/sql-events?target_name=mysql-local&client_addr=127.0.0.1:51042&fingerprint=select%20filtered",
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

    #[tokio::test]
    async fn sql_event_detail_returns_full_event() {
        let mut event = test_event("evt_1");
        event.error = Some(ErrorSummary {
            code: Some("1064".to_owned()),
            sql_state: Some("42000".to_owned()),
            message: "syntax error".to_owned(),
            metadata: Some(ProtocolMetadata {
                protocol: ProtocolName("mysql".to_owned()),
                fields: vec![MetadataField {
                    key: "severity".to_owned(),
                    value: MetadataValue::String("error".to_owned()),
                }],
            }),
        });

        let (status, json, has_request_id) =
            get_json(app_with_events(vec![event]), "/api/v1/sql-events/evt_1").await;

        assert_eq!(status, StatusCode::OK);
        assert!(has_request_id);
        assert_eq!(json["id"], "evt_1");
        assert_eq!(json["target_name"], "mysql-local");
        assert_eq!(json["normalized_sql"], "select * from users where id = ?");
        assert_eq!(json["parameters"][0]["index"], 0);
        assert_eq!(json["parameters"][0]["name"], "id");
        assert_eq!(json["parameters"][0]["value"]["type"], "integer");
        assert_eq!(json["parameters"][0]["value"]["value"], 42);
        assert_eq!(json["timings"]["started_at"], "2026-07-07T09:00:00Z");
        assert_eq!(json["timings"]["duration_ms"], 3);
        assert_eq!(json["rows"]["returned"], 1);
        assert_eq!(json["error"]["code"], "1064");
        assert_eq!(json["error"]["sql_state"], "42000");
        assert_eq!(json["error"]["metadata"]["mysql"]["severity"], "error");
        assert_eq!(json["metadata"]["mysql"]["statement_id"], 12);
    }

    #[tokio::test]
    async fn sql_event_detail_returns_not_found_for_missing_event() {
        let (status, json, has_request_id) =
            get_json(app_with_events(Vec::new()), "/api/v1/sql-events/missing").await;

        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(has_request_id);
        assert_eq!(json["error"]["code"], "NOT_FOUND");
        assert_eq!(json["error"]["message"], "SQL event not found");
        assert_eq!(json["error"]["details"]["id"], "missing");
    }
}
