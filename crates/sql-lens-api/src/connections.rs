use std::num::NonZeroUsize;

use axum::{
    Extension, Json, Router,
    extract::{Path, Query},
    routing::get,
};
use serde::{Deserialize, Serialize};
use sql_lens_core::{ConnectionId, ConnectionInfo, ConnectionState, Timestamp};

use crate::{ApiState, api_error::ApiEndpointError};

pub const CONNECTIONS_PATH: &str = "/api/v1/connections";
pub const CONNECTION_DETAIL_PATH: &str = "/api/v1/connections/{id}";
const DEFAULT_LIMIT: usize = 100;
const MAX_LIMIT: usize = 500;

pub(crate) fn routes() -> Router {
    Router::new()
        .route(CONNECTIONS_PATH, get(list_connections))
        .route(CONNECTION_DETAIL_PATH, get(get_connection_detail))
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct ConnectionListQueryParams {
    limit: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectionListResponse {
    pub items: Vec<ConnectionResponse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectionResponse {
    pub id: String,
    pub target_name: Option<String>,
    pub protocol: String,
    pub database_type: String,
    pub client_addr: String,
    pub backend_addr: String,
    pub user: Option<String>,
    pub database: Option<String>,
    pub state: String,
    pub connected_at: String,
    pub closed_at: Option<String>,
    pub last_activity_at: Option<String>,
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub query_count: u64,
}

async fn list_connections(
    Extension(state): Extension<ApiState>,
    Query(params): Query<ConnectionListQueryParams>,
) -> Result<Json<ConnectionListResponse>, ApiEndpointError> {
    let limit = parse_limit(params.limit)?;
    let connections = {
        let connection_store = state.connection_store();
        let store = connection_store.read().await;
        store.list_recent(limit)
    };

    Ok(Json(ConnectionListResponse {
        items: connections.iter().map(ConnectionResponse::from).collect(),
    }))
}

async fn get_connection_detail(
    Extension(state): Extension<ApiState>,
    Path(id): Path<String>,
) -> Result<Json<ConnectionResponse>, ApiEndpointError> {
    let connection = {
        let connection_store = state.connection_store();
        let store = connection_store.read().await;
        store.get(&ConnectionId(id.clone())).cloned()
    }
    .ok_or_else(|| ApiEndpointError::not_found("Connection not found", "id", id))?;

    Ok(Json(ConnectionResponse::from(&connection)))
}

fn parse_limit(limit: Option<usize>) -> Result<NonZeroUsize, ApiEndpointError> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT);
    NonZeroUsize::new(limit)
        .ok_or_else(|| ApiEndpointError::bad_request("limit must be greater than zero", "limit"))
}

impl From<&ConnectionInfo> for ConnectionResponse {
    fn from(connection: &ConnectionInfo) -> Self {
        Self {
            id: connection.id.0.clone(),
            target_name: connection.target_name.clone(),
            protocol: connection.protocol.0.clone(),
            database_type: connection.database_type.0.clone(),
            client_addr: connection.client_addr.clone(),
            backend_addr: connection.backend_addr.clone(),
            user: connection.user.clone(),
            database: connection.database.clone(),
            state: connection_state_name(connection.state).to_owned(),
            connected_at: timestamp_value(&connection.connected_at),
            closed_at: connection.closed_at.as_ref().map(timestamp_value),
            last_activity_at: connection.last_activity_at.as_ref().map(timestamp_value),
            bytes_in: connection.bytes_in,
            bytes_out: connection.bytes_out,
            query_count: connection.query_count,
        }
    }
}

fn timestamp_value(timestamp: &Timestamp) -> String {
    timestamp.0.clone()
}

fn connection_state_name(state: ConnectionState) -> &'static str {
    match state {
        ConnectionState::Created => "created",
        ConnectionState::BackendConnected => "backend_connected",
        ConnectionState::HandshakeSeen => "handshake_seen",
        ConnectionState::Authenticating => "authenticating",
        ConnectionState::Ready => "ready",
        ConnectionState::CommandInFlight => "command_in_flight",
        ConnectionState::Closing => "closing",
        ConnectionState::Closed => "closed",
        ConnectionState::Failed => "failed",
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
    use sql_lens_core::{DatabaseType, ProtocolName};
    use sql_lens_storage::{ConnectionStore, RingBufferStore};
    use tower::ServiceExt;

    use super::*;
    use crate::{REQUEST_ID_HEADER, router_with_state};

    fn capacity(value: usize) -> NonZeroUsize {
        NonZeroUsize::new(value).expect("test capacity should be non-zero")
    }

    fn test_connection(id: &str, state: ConnectionState) -> ConnectionInfo {
        ConnectionInfo {
            id: ConnectionId(id.to_owned()),
            target_name: Some("mysql-local".to_owned()),
            protocol: ProtocolName("mysql".to_owned()),
            database_type: DatabaseType("mysql".to_owned()),
            client_addr: "127.0.0.1:51000".to_owned(),
            backend_addr: "127.0.0.1:3306".to_owned(),
            user: Some("app".to_owned()),
            database: Some("app".to_owned()),
            state,
            connected_at: Timestamp("2026-07-07T09:00:00Z".to_owned()),
            closed_at: None,
            last_activity_at: Some(Timestamp("2026-07-07T09:00:00Z".to_owned())),
            bytes_in: 128,
            bytes_out: 256,
            query_count: 3,
        }
    }

    fn app_with_connections(connections: Vec<ConnectionInfo>) -> Router {
        let event_store = RingBufferStore::new(capacity(10));
        let mut connection_store = ConnectionStore::new(capacity(10));
        for connection in connections {
            connection_store.upsert(connection);
        }

        router_with_state(ApiState::with_stores(event_store, connection_store))
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
    async fn connection_list_returns_active_and_closed_connections() {
        let mut closed = test_connection("conn_2", ConnectionState::Closed);
        closed.closed_at = Some(Timestamp("2026-07-07T09:01:00Z".to_owned()));

        let (status, json, has_request_id) = get_json(
            app_with_connections(vec![
                test_connection("conn_1", ConnectionState::Ready),
                closed,
            ]),
            CONNECTIONS_PATH,
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert!(has_request_id);
        assert_eq!(json["items"][0]["id"], "conn_2");
        assert_eq!(json["items"][0]["target_name"], "mysql-local");
        assert_eq!(json["items"][0]["state"], "closed");
        assert_eq!(json["items"][0]["closed_at"], "2026-07-07T09:01:00Z");
        assert_eq!(json["items"][1]["id"], "conn_1");
        assert_eq!(json["items"][1]["state"], "ready");
    }

    #[tokio::test]
    async fn connection_detail_returns_existing_connection() {
        let (status, json, has_request_id) = get_json(
            app_with_connections(vec![test_connection("conn_1", ConnectionState::Ready)]),
            "/api/v1/connections/conn_1",
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert!(has_request_id);
        assert_eq!(json["id"], "conn_1");
        assert_eq!(json["target_name"], "mysql-local");
        assert_eq!(json["protocol"], "mysql");
        assert_eq!(json["database_type"], "mysql");
        assert_eq!(json["client_addr"], "127.0.0.1:51000");
        assert_eq!(json["backend_addr"], "127.0.0.1:3306");
        assert_eq!(json["state"], "ready");
        assert_eq!(json["bytes_in"], 128);
        assert_eq!(json["bytes_out"], 256);
        assert_eq!(json["query_count"], 3);
    }

    #[tokio::test]
    async fn connection_detail_returns_not_found_for_missing_connection() {
        let (status, json, has_request_id) = get_json(
            app_with_connections(Vec::new()),
            "/api/v1/connections/missing",
        )
        .await;

        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(has_request_id);
        assert_eq!(json["error"]["code"], "NOT_FOUND");
        assert_eq!(json["error"]["message"], "Connection not found");
        assert_eq!(json["error"]["details"]["id"], "missing");
    }

    #[tokio::test]
    async fn connection_list_rejects_zero_limit() {
        let (status, json, has_request_id) = get_json(
            app_with_connections(Vec::new()),
            "/api/v1/connections?limit=0",
        )
        .await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(has_request_id);
        assert_eq!(json["error"]["code"], "BAD_REQUEST");
        assert_eq!(json["error"]["details"]["field"], "limit");
    }
}
