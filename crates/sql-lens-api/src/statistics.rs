use axum::{Extension, Json, Router, extract::Query, routing::get};
use serde::{Deserialize, Serialize};

use crate::{ApiState, api_error::ApiEndpointError};

pub const STATISTICS_PATH: &str = "/api/v1/statistics";
const LIVE_WINDOW: &str = "1m";

pub(crate) fn routes() -> Router {
    Router::new().route(STATISTICS_PATH, get(get_statistics))
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct StatisticsQueryParams {
    window: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatisticsResponse {
    pub window: String,
    pub qps: f64,
    pub error_rate: f64,
    pub slow_count: u64,
    pub latency_ms: LatencyPercentilesResponse,
    pub active_connections: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LatencyPercentilesResponse {
    pub p50: f64,
    pub p95: f64,
    pub p99: f64,
}

async fn get_statistics(
    Extension(state): Extension<ApiState>,
    Query(params): Query<StatisticsQueryParams>,
) -> Result<Json<StatisticsResponse>, ApiEndpointError> {
    validate_window(params.window.as_deref())?;

    let snapshot = {
        let live_statistics = state.live_statistics();
        let mut statistics = live_statistics.write().await;
        statistics.snapshot()
    };

    let error_rate = if snapshot.total_events == 0 {
        0.0
    } else {
        snapshot.error_events as f64 / snapshot.total_events as f64
    };

    Ok(Json(StatisticsResponse {
        window: LIVE_WINDOW.to_owned(),
        qps: snapshot.qps,
        error_rate,
        slow_count: snapshot.slow_events,
        latency_ms: LatencyPercentilesResponse {
            p50: snapshot.latency_percentiles.p50,
            p95: snapshot.latency_percentiles.p95,
            p99: snapshot.latency_percentiles.p99,
        },
        active_connections: snapshot.active_connections,
    }))
}

fn validate_window(window: Option<&str>) -> Result<(), ApiEndpointError> {
    match window {
        None | Some("1m" | "60s") => Ok(()),
        Some(_) => Err(ApiEndpointError::bad_request(
            "window must be one of 1m, 60s",
            "window",
        )),
    }
}

#[cfg(test)]
mod tests {
    use std::{num::NonZeroUsize, time::Instant};

    use axum::{
        Router,
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use serde_json::Value;
    use sql_lens_core::{
        CaptureStatus, ConnectionId, DatabaseType, DurationMillis, ProtocolMetadata, ProtocolName,
        QueryTiming, SqlEvent, SqlEventId, SqlEventKind, Timestamp,
    };
    use sql_lens_storage::{ConnectionStore, LiveStatistics, RingBufferStore};
    use tower::ServiceExt;

    use crate::{ApiState, REQUEST_ID_HEADER, router_with_state};

    use super::{STATISTICS_PATH, StatisticsResponse};

    #[tokio::test]
    async fn statistics_returns_empty_state() {
        let app = app_with_statistics(LiveStatistics::new());

        let (status, response, has_request_id) = get_json(app, STATISTICS_PATH).await;

        assert_eq!(status, StatusCode::OK);
        assert!(has_request_id);
        assert_eq!(
            response,
            StatisticsResponse {
                window: "1m".to_owned(),
                qps: 0.0,
                error_rate: 0.0,
                slow_count: 0,
                latency_ms: super::LatencyPercentilesResponse {
                    p50: 0.0,
                    p95: 0.0,
                    p99: 0.0,
                },
                active_connections: 0,
            }
        );
    }

    #[tokio::test]
    async fn statistics_returns_populated_state() {
        let mut statistics = LiveStatistics::new();
        let now = Instant::now();
        let conn_1 = ConnectionId("conn_1".to_owned());

        statistics.record_connection_opened(conn_1.clone());
        statistics.record_sql_event_at(
            &test_event("evt_1", CaptureStatus::Ok, DurationMillis(10), &conn_1),
            now,
        );
        statistics.record_sql_event_at(
            &test_event("evt_2", CaptureStatus::Slow, DurationMillis(20), &conn_1),
            now,
        );
        statistics.record_sql_event_at(
            &test_event("evt_3", CaptureStatus::Error, DurationMillis(40), &conn_1),
            now,
        );
        let app = app_with_statistics(statistics);

        let (status, response, has_request_id) = get_json(app, STATISTICS_PATH).await;

        assert_eq!(status, StatusCode::OK);
        assert!(has_request_id);
        assert_eq!(response.window, "1m");
        assert!((response.qps - 0.05).abs() < f64::EPSILON);
        assert!((response.error_rate - (1.0 / 3.0)).abs() < f64::EPSILON);
        assert_eq!(response.slow_count, 1);
        assert_eq!(response.latency_ms.p50, 20.0);
        assert_eq!(response.latency_ms.p95, 40.0);
        assert_eq!(response.latency_ms.p99, 40.0);
        assert_eq!(response.active_connections, 1);
    }

    #[tokio::test]
    async fn statistics_rejects_invalid_window() {
        let app = app_with_statistics(LiveStatistics::new());

        let (status, json, has_request_id) = get_value(app, "/api/v1/statistics?window=5m").await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(has_request_id);
        assert_eq!(json["error"]["code"], "BAD_REQUEST");
        assert_eq!(json["error"]["details"]["field"], "window");
    }

    fn app_with_statistics(statistics: LiveStatistics) -> Router {
        router_with_state(ApiState::with_all_stores(
            RingBufferStore::new(capacity(10)),
            ConnectionStore::new(capacity(10)),
            statistics,
        ))
    }

    fn capacity(value: usize) -> NonZeroUsize {
        NonZeroUsize::new(value).expect("test capacity should be non-zero")
    }

    async fn get_json(app: Router, uri: &str) -> (StatusCode, StatisticsResponse, bool) {
        let (status, value, has_request_id) = get_value(app, uri).await;
        let response = serde_json::from_value(value).expect("response should match statistics");

        (status, response, has_request_id)
    }

    async fn get_value(app: Router, uri: &str) -> (StatusCode, Value, bool) {
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

    fn test_event(
        id: &str,
        status: CaptureStatus,
        duration: DurationMillis,
        connection_id: &ConnectionId,
    ) -> SqlEvent {
        SqlEvent {
            id: SqlEventId(id.to_owned()),
            timestamp: Timestamp("2026-07-07T09:00:00Z".to_owned()),
            protocol: ProtocolName("mysql".to_owned()),
            database_type: DatabaseType("mysql".to_owned()),
            connection_id: connection_id.clone(),
            client_addr: "127.0.0.1:51000".to_owned(),
            backend_addr: "127.0.0.1:3306".to_owned(),
            user: Some("app".to_owned()),
            database: Some("app".to_owned()),
            kind: SqlEventKind::Query,
            status,
            duration,
            original_sql: "SELECT 1".to_owned(),
            normalized_sql: Some("select 1".to_owned()),
            expanded_sql: None,
            fingerprint: Some("select ?".to_owned()),
            parameters: Vec::new(),
            result: None,
            error: None,
            timings: QueryTiming {
                started_at: Timestamp("2026-07-07T09:00:00Z".to_owned()),
                ended_at: Some(Timestamp("2026-07-07T09:00:00Z".to_owned())),
                duration,
            },
            metadata: ProtocolMetadata {
                protocol: ProtocolName("mysql".to_owned()),
                fields: Vec::new(),
            },
        }
    }
}
