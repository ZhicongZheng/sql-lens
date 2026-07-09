use axum::{Json, Router, routing::get};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub const PROTOCOLS_PATH: &str = "/api/v1/protocols";

pub(crate) fn routes() -> Router {
    Router::new().route(PROTOCOLS_PATH, get(list_protocols))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ProtocolListResponse {
    pub items: Vec<ProtocolResponse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ProtocolResponse {
    pub name: String,
    pub status: String,
    pub databases: Vec<String>,
}

async fn list_protocols() -> Json<ProtocolListResponse> {
    Json(ProtocolListResponse {
        items: vec![
            protocol(
                "mysql",
                "supported",
                &["mysql", "starrocks", "tidb", "doris"],
            ),
            protocol("postgresql", "planned", &["postgresql"]),
            protocol("clickhouse", "planned", &["clickhouse"]),
            protocol("sqlite", "planned", &["sqlite"]),
        ],
    })
}

fn protocol(name: &str, status: &str, databases: &[&str]) -> ProtocolResponse {
    ProtocolResponse {
        name: name.to_owned(),
        status: status.to_owned(),
        databases: databases
            .iter()
            .map(|database| (*database).to_owned())
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    use crate::{PROTOCOLS_PATH, ProtocolListResponse, REQUEST_ID_HEADER, router};

    #[tokio::test]
    async fn protocols_endpoint_returns_supported_and_planned_protocols() {
        let response = router()
            .oneshot(
                Request::builder()
                    .uri(PROTOCOLS_PATH)
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should be handled");

        assert_eq!(response.status(), StatusCode::OK);
        assert!(
            response.headers().contains_key(REQUEST_ID_HEADER),
            "protocol responses should include request IDs"
        );

        let body = to_bytes(response.into_body(), 4096)
            .await
            .expect("response body should be readable");
        let payload: ProtocolListResponse =
            serde_json::from_slice(&body).expect("protocol response should be JSON");

        let mysql = payload
            .items
            .iter()
            .find(|item| item.name == "mysql")
            .expect("mysql protocol should be listed");

        assert_eq!(mysql.status, "supported");
        assert_eq!(mysql.databases, ["mysql", "starrocks", "tidb", "doris"]);

        assert!(payload.items.iter().any(|item| item.name == "postgresql"
            && item.status == "planned"
            && item.databases == ["postgresql"]));
        assert!(payload.items.iter().any(|item| item.name == "clickhouse"
            && item.status == "planned"
            && item.databases == ["clickhouse"]));
        assert!(payload.items.iter().any(|item| item.name == "sqlite"
            && item.status == "planned"
            && item.databases == ["sqlite"]));
    }
}
