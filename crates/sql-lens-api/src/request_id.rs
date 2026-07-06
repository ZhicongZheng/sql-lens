use std::sync::atomic::{AtomicU64, Ordering};

use axum::{
    extract::Request,
    http::{HeaderName, HeaderValue},
    middleware::Next,
    response::Response,
};

pub const REQUEST_ID_HEADER: &str = "x-request-id";

static NEXT_REQUEST_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestId(HeaderValue);

impl RequestId {
    pub fn as_header_value(&self) -> &HeaderValue {
        &self.0
    }
}

pub(crate) async fn attach_request_id(mut request: Request, next: Next) -> Response {
    let request_id = request
        .headers()
        .get(REQUEST_ID_HEADER)
        .cloned()
        .unwrap_or_else(next_request_id);

    request
        .extensions_mut()
        .insert(RequestId(request_id.clone()));

    let mut response = next.run(request).await;
    response
        .headers_mut()
        .insert(request_id_header_name(), request_id);
    response
}

fn next_request_id() -> HeaderValue {
    let id = NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed);
    HeaderValue::from_str(&format!("sql-lens-{id:016x}"))
        .expect("generated request IDs contain only valid header value bytes")
}

fn request_id_header_name() -> HeaderName {
    HeaderName::from_static(REQUEST_ID_HEADER)
}

#[cfg(test)]
mod tests {
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode},
    };
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
            .expect("response should contain request id");

        assert!(
            request_id
                .to_str()
                .expect("generated request ID should be ASCII")
                .starts_with("sql-lens-")
        );
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

        assert_eq!(
            response.headers().get(REQUEST_ID_HEADER),
            Some(&"client-request-42".parse().expect("valid header value"))
        );
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
}
