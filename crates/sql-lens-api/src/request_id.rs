use std::sync::atomic::{AtomicU64, Ordering};

use axum::{
    extract::Request,
    http::{HeaderName, HeaderValue},
    middleware::Next,
    response::Response,
};

use crate::api_error;

pub const REQUEST_ID_HEADER: &str = "x-request-id";

static NEXT_REQUEST_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestId {
    header_value: HeaderValue,
    value: String,
}

impl RequestId {
    fn from_header_value(header_value: HeaderValue) -> Option<Self> {
        let value = header_value.to_str().ok()?.to_owned();

        Some(Self {
            header_value,
            value,
        })
    }

    pub fn as_header_value(&self) -> &HeaderValue {
        &self.header_value
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.value
    }
}

pub(crate) async fn attach_request_id(mut request: Request, next: Next) -> Response {
    let request_id = request
        .headers()
        .get(REQUEST_ID_HEADER)
        .cloned()
        .and_then(RequestId::from_header_value)
        .unwrap_or_else(next_request_id);

    request.extensions_mut().insert(request_id.clone());

    let mut response = api_error::with_request_id(next.run(request).await, &request_id);
    response.headers_mut().insert(
        request_id_header_name(),
        request_id.as_header_value().clone(),
    );
    response
}

fn next_request_id() -> RequestId {
    let id = NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed);
    let value = format!("sql-lens-{id:016x}");
    let header_value = HeaderValue::from_str(&value)
        .expect("generated request IDs contain only valid header value bytes");

    RequestId {
        header_value,
        value,
    }
}

fn request_id_header_name() -> HeaderName {
    HeaderName::from_static(REQUEST_ID_HEADER)
}

#[cfg(test)]
mod tests {
    use axum::{
        Router,
        body::{Body, to_bytes},
        http::{HeaderValue, Request, StatusCode},
    };
    use serde_json::Value;
    use tower::ServiceExt;

    use super::REQUEST_ID_HEADER;
    use crate::router;

    #[tokio::test]
    async fn generated_request_id_is_added_to_response() {
        let response = router()
            .oneshot(
                Request::builder()
                    .uri("/missing")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should be handled");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let request_id = response
            .headers()
            .get(REQUEST_ID_HEADER)
            .expect("response should contain request id")
            .to_str()
            .expect("generated request ID should be ASCII")
            .to_owned();

        assert!(request_id.starts_with("sql-lens-"));

        let json = response_json(response).await;
        assert_eq!(json["error"]["code"], "NOT_FOUND");
        assert_eq!(json["error"]["request_id"], request_id);
    }

    #[tokio::test]
    async fn incoming_request_id_is_preserved() {
        let response = router()
            .oneshot(
                Request::builder()
                    .uri("/missing")
                    .header(REQUEST_ID_HEADER, "client-request-42")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should be handled");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            response.headers().get(REQUEST_ID_HEADER),
            Some(&"client-request-42".parse().expect("valid header value"))
        );

        let json = response_json(response).await;
        assert_eq!(json["error"]["request_id"], "client-request-42");
    }

    #[tokio::test]
    async fn invalid_incoming_request_id_is_replaced() {
        let response = router()
            .oneshot(
                Request::builder()
                    .uri("/missing")
                    .header(
                        REQUEST_ID_HEADER,
                        HeaderValue::from_bytes(b"\xff").expect("opaque header should build"),
                    )
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should be handled");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let request_id = response
            .headers()
            .get(REQUEST_ID_HEADER)
            .expect("response should contain request id")
            .to_str()
            .expect("generated request ID should be ASCII")
            .to_owned();

        assert!(request_id.starts_with("sql-lens-"));

        let json = response_json(response).await;
        assert_eq!(json["error"]["request_id"], request_id);
    }

    #[tokio::test]
    async fn request_id_is_available_to_handlers() {
        let app = Router::new()
            .route(
                "/request-id",
                axum::routing::get(
                    |axum::Extension(request_id): axum::Extension<super::RequestId>| async move {
                        request_id
                            .as_header_value()
                            .to_str()
                            .expect("request ID should be ASCII")
                            .to_owned()
                    },
                ),
            )
            .layer(axum::middleware::from_fn(super::attach_request_id));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/request-id")
                    .header(REQUEST_ID_HEADER, "client-request-43")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should be handled");

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(REQUEST_ID_HEADER),
            Some(&"client-request-43".parse().expect("valid header value"))
        );
    }

    async fn response_json(response: axum::response::Response) -> Value {
        let body = to_bytes(response.into_body(), 4096)
            .await
            .expect("response body should be readable");
        serde_json::from_slice(&body).expect("response should be JSON")
    }
}
