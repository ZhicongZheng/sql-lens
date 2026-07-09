use axum::{Extension, Json, Router, routing::post};
use serde::{Deserialize, Serialize};
use sql_lens_core::SqlEventId;
use utoipa::ToSchema;

use crate::{ApiState, api_error::ApiEndpointError};

pub const REPLAY_PREVIEW_PATH: &str = "/api/v1/replay/preview";
const MUTATION_WARNING: &str =
    "SQL may modify data or schema and will require explicit confirmation before execution.";

pub(crate) fn routes() -> Router {
    Router::new().route(REPLAY_PREVIEW_PATH, post(preview_replay))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ReplayPreviewRequest {
    pub event_id: Option<String>,
    pub sql: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ReplayPreviewResponse {
    pub source: String,
    pub event_id: Option<String>,
    pub sql: String,
    pub is_mutation: bool,
    pub warning: Option<String>,
}

async fn preview_replay(
    Extension(state): Extension<ApiState>,
    Json(request): Json<ReplayPreviewRequest>,
) -> Result<Json<ReplayPreviewResponse>, ApiEndpointError> {
    let source = ReplayPreviewSource::try_from_request(request)?;
    let preview = match source {
        ReplayPreviewSource::Event { event_id } => preview_event(&state, event_id).await?,
        ReplayPreviewSource::RawSql { sql } => {
            preview_sql(ReplayPreviewSourceKind::RawSql, None, sql)?
        }
    };

    Ok(Json(preview))
}

async fn preview_event(
    state: &ApiState,
    event_id: String,
) -> Result<ReplayPreviewResponse, ApiEndpointError> {
    let event = state
        .event_reader()
        .get_detail(&SqlEventId(event_id.clone()))
        .await?
        .ok_or_else(|| {
            ApiEndpointError::not_found("SQL event not found", "event_id", event_id.clone())
        })?;
    let sql = replay_sql_from_event_detail(&event);

    preview_sql(ReplayPreviewSourceKind::Event, Some(event_id), sql)
}

fn replay_sql_from_event_detail(event: &crate::SqlEventDetailResponse) -> String {
    event
        .expanded_sql
        .clone()
        .unwrap_or_else(|| event.original_sql.clone())
}

fn preview_sql(
    source: ReplayPreviewSourceKind,
    event_id: Option<String>,
    sql: String,
) -> Result<ReplayPreviewResponse, ApiEndpointError> {
    if sql.trim().is_empty() {
        return Err(ApiEndpointError::bad_request(
            "sql must not be empty",
            "sql",
        ));
    }

    let is_mutation = is_mutation_sql(&sql);

    Ok(ReplayPreviewResponse {
        source: source.as_str().to_owned(),
        event_id,
        sql,
        is_mutation,
        warning: is_mutation.then(|| MUTATION_WARNING.to_owned()),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ReplayPreviewSource {
    Event { event_id: String },
    RawSql { sql: String },
}

impl ReplayPreviewSource {
    fn try_from_request(request: ReplayPreviewRequest) -> Result<Self, ApiEndpointError> {
        let event_id = request.event_id.map(|value| value.trim().to_owned());
        let sql = request.sql.map(|value| value.trim().to_owned());

        match (event_id, sql) {
            (Some(event_id), None) if !event_id.is_empty() => Ok(Self::Event { event_id }),
            (None, Some(sql)) => Ok(Self::RawSql { sql }),
            (Some(event_id), Some(_)) if !event_id.is_empty() => {
                Err(ApiEndpointError::bad_request(
                    "replay preview accepts either event_id or sql, not both",
                    "source",
                ))
            }
            _ => Err(ApiEndpointError::bad_request(
                "replay preview requires event_id or sql",
                "source",
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReplayPreviewSourceKind {
    Event,
    RawSql,
}

impl ReplayPreviewSourceKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Event => "event",
            Self::RawSql => "raw_sql",
        }
    }
}

fn is_mutation_sql(sql: &str) -> bool {
    let Some(keyword) = first_sql_keyword(sql) else {
        return true;
    };

    !matches!(
        keyword.as_str(),
        "select" | "show" | "describe" | "desc" | "explain"
    )
}

fn first_sql_keyword(sql: &str) -> Option<String> {
    let bytes = sql.as_bytes();
    let mut index = 0;

    loop {
        index = skip_ascii_whitespace(bytes, index);

        if bytes.get(index..index + 2) == Some(b"--") {
            index = skip_line_comment(bytes, index + 2);
            continue;
        }

        if bytes.get(index) == Some(&b'#') {
            index = skip_line_comment(bytes, index + 1);
            continue;
        }

        if bytes.get(index..index + 2) == Some(b"/*") {
            index = skip_block_comment(bytes, index + 2);
            continue;
        }

        break;
    }

    let start = index;
    while index < bytes.len() && bytes[index].is_ascii_alphabetic() {
        index += 1;
    }

    (index > start).then(|| sql[start..index].to_ascii_lowercase())
}

fn skip_ascii_whitespace(bytes: &[u8], mut index: usize) -> usize {
    while index < bytes.len() && bytes[index].is_ascii_whitespace() {
        index += 1;
    }

    index
}

fn skip_line_comment(bytes: &[u8], mut index: usize) -> usize {
    while index < bytes.len() && bytes[index] != b'\n' {
        index += 1;
    }

    index
}

fn skip_block_comment(bytes: &[u8], mut index: usize) -> usize {
    while index + 1 < bytes.len() {
        if bytes[index] == b'*' && bytes[index + 1] == b'/' {
            return index + 2;
        }
        index += 1;
    }

    bytes.len()
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use serde_json::{Value, json};
    use sql_lens_core::{SqlEvent, SqlEventId};
    use sql_lens_storage::RingBufferStore;
    use tower::ServiceExt;

    use super::*;
    use crate::{
        REQUEST_ID_HEADER, router_with_state,
        test_support::{sqlite_api_state_with_events, test_event},
    };

    fn capacity(value: usize) -> NonZeroUsize {
        NonZeroUsize::new(value).expect("test capacity should be non-zero")
    }

    fn app_with_events(events: Vec<SqlEvent>) -> Router {
        let mut store = RingBufferStore::new(capacity(10));
        for event in events {
            store.append(event);
        }

        router_with_state(ApiState::new(store))
    }

    fn app_with_sqlite_events(events: Vec<SqlEvent>) -> Router {
        router_with_state(sqlite_api_state_with_events(events))
    }

    async fn post_json(app: Router, body: Value) -> (StatusCode, Value, bool) {
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(REPLAY_PREVIEW_PATH)
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
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
    async fn replay_preview_uses_expanded_sql_for_event_source() {
        let event = test_event("evt_1");

        let (status, json, has_request_id) =
            post_json(app_with_events(vec![event]), json!({ "event_id": "evt_1" })).await;

        assert_eq!(status, StatusCode::OK);
        assert!(has_request_id);
        assert_eq!(json["source"], "event");
        assert_eq!(json["event_id"], "evt_1");
        assert_eq!(json["sql"], "SELECT * FROM users WHERE id = 42");
        assert_eq!(json["is_mutation"], false);
        assert!(json["warning"].is_null());
    }

    #[tokio::test]
    async fn sqlite_backed_replay_preview_reads_persisted_event_source() {
        let event = test_event("evt_sqlite_replay");

        let (status, json, has_request_id) = post_json(
            app_with_sqlite_events(vec![event]),
            json!({ "event_id": "evt_sqlite_replay" }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert!(has_request_id);
        assert_eq!(json["source"], "event");
        assert_eq!(json["event_id"], "evt_sqlite_replay");
        assert_eq!(json["sql"], "SELECT * FROM users WHERE id = 42");
    }

    #[tokio::test]
    async fn replay_preview_falls_back_to_original_sql_for_event_source() {
        let mut event = test_event("evt_1");
        event.expanded_sql = None;

        let (status, json, _) =
            post_json(app_with_events(vec![event]), json!({ "event_id": "evt_1" })).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["sql"], "SELECT * FROM users WHERE id = ?");
        assert_eq!(json["is_mutation"], false);
    }

    #[tokio::test]
    async fn replay_preview_accepts_raw_sql_and_flags_mutations() {
        let (status, json, has_request_id) = post_json(
            app_with_events(Vec::new()),
            json!({ "sql": "UPDATE users SET name = 'a'" }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert!(has_request_id);
        assert_eq!(json["source"], "raw_sql");
        assert!(json["event_id"].is_null());
        assert_eq!(json["sql"], "UPDATE users SET name = 'a'");
        assert_eq!(json["is_mutation"], true);
        assert_eq!(json["warning"], MUTATION_WARNING);
    }

    #[tokio::test]
    async fn replay_preview_treats_common_read_only_sql_as_non_mutating() {
        for sql in [
            "SELECT 1",
            " show databases",
            "-- comment\nEXPLAIN SELECT 1",
            "/* comment */ DESCRIBE users",
            "# comment\nDESC users",
        ] {
            assert!(!is_mutation_sql(sql), "{sql} should be read-only");
        }
    }

    #[tokio::test]
    async fn replay_preview_rejects_missing_or_ambiguous_sources() {
        for body in [json!({}), json!({ "event_id": "evt_1", "sql": "SELECT 1" })] {
            let (status, json, has_request_id) = post_json(app_with_events(Vec::new()), body).await;

            assert_eq!(status, StatusCode::BAD_REQUEST);
            assert!(has_request_id);
            assert_eq!(json["error"]["code"], "BAD_REQUEST");
            assert_eq!(json["error"]["details"]["field"], "source");
        }
    }

    #[tokio::test]
    async fn replay_preview_rejects_empty_raw_sql() {
        let (status, json, has_request_id) =
            post_json(app_with_events(Vec::new()), json!({ "sql": "  " })).await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(has_request_id);
        assert_eq!(json["error"]["code"], "BAD_REQUEST");
        assert_eq!(json["error"]["details"]["field"], "sql");
    }

    #[tokio::test]
    async fn replay_preview_rejects_missing_event_id() {
        let (status, json, has_request_id) = post_json(
            app_with_events(Vec::new()),
            json!({ "event_id": "missing" }),
        )
        .await;

        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(has_request_id);
        assert_eq!(json["error"]["code"], "NOT_FOUND");
        assert_eq!(json["error"]["message"], "SQL event not found");
        assert_eq!(json["error"]["details"]["event_id"], "missing");
    }

    #[tokio::test]
    async fn replay_preview_rejects_event_with_empty_sql() {
        let mut event = test_event("evt_1");
        event.expanded_sql = None;
        event.original_sql = " ".to_owned();

        let (status, json, _) =
            post_json(app_with_events(vec![event]), json!({ "event_id": "evt_1" })).await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(json["error"]["details"]["field"], "sql");
    }

    #[tokio::test]
    async fn replay_preview_does_not_modify_event_storage() {
        let event = test_event("evt_1");
        let mut store = RingBufferStore::new(capacity(10));
        store.append(event.clone());
        let state = ApiState::new(store);
        let app = router_with_state(state.clone());

        let (status, _, _) = post_json(app.clone(), json!({ "event_id": "evt_1" })).await;
        assert_eq!(status, StatusCode::OK);

        let state_event_id = SqlEventId("evt_1".to_owned());
        let event_store = state.event_store();
        let store = event_store.read().await;

        assert_eq!(store.get(&state_event_id), Some(&event));
    }
}
