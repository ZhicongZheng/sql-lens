use std::time::Instant;

use axum::{Extension, Json, Router, routing::get};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub const HEALTH_PATH: &str = "/api/v1/health";
const HEALTH_STATUS_OK: &str = "ok";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_ms: u64,
}

#[derive(Debug, Clone)]
pub struct HealthState {
    started_at: Instant,
    version: &'static str,
}

impl HealthState {
    pub fn new() -> Self {
        Self {
            started_at: Instant::now(),
            version: env!("CARGO_PKG_VERSION"),
        }
    }

    pub fn snapshot(&self) -> HealthResponse {
        HealthResponse {
            status: HEALTH_STATUS_OK.to_owned(),
            version: self.version.to_owned(),
            uptime_ms: self.uptime_ms(),
        }
    }

    fn uptime_ms(&self) -> u64 {
        let uptime = self.started_at.elapsed().as_millis();
        uptime.try_into().unwrap_or(u64::MAX)
    }
}

impl Default for HealthState {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) fn routes() -> Router {
    Router::new()
        .route(HEALTH_PATH, get(health))
        .layer(Extension(HealthState::new()))
}

async fn health(Extension(state): Extension<HealthState>) -> Json<HealthResponse> {
    Json(state.snapshot())
}

#[cfg(test)]
mod tests {
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    use crate::{HEALTH_PATH, HealthResponse, REQUEST_ID_HEADER, router};

    #[tokio::test]
    async fn health_endpoint_returns_documented_schema() {
        let response = router()
            .oneshot(
                Request::builder()
                    .uri(HEALTH_PATH)
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should be handled");

        assert_eq!(response.status(), StatusCode::OK);
        assert!(
            response.headers().contains_key(REQUEST_ID_HEADER),
            "health responses should include request IDs"
        );

        let body = to_bytes(response.into_body(), 1024)
            .await
            .expect("response body should be readable");
        let payload: HealthResponse =
            serde_json::from_slice(&body).expect("health response should be JSON");

        assert_eq!(payload.status, "ok");
        assert_eq!(payload.version, env!("CARGO_PKG_VERSION"));
    }
}
