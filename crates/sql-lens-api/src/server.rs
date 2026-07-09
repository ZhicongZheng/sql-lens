use std::{error::Error, fmt, future::Future, net::SocketAddr};

use axum::{
    Router,
    extract::OriginalUri,
    http::{
        HeaderValue, Method,
        header::{self, InvalidHeaderValue},
    },
    middleware,
};
use sql_lens_config::WebConfig;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;

use crate::{
    ApiState, REQUEST_ID_HEADER, api_error::ApiEndpointError, connections, export, health,
    protocols, replay, request_id::attach_request_id, sql_events, statistics, websocket,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpServerConfig {
    pub listen: String,
    pub cors_origins: Vec<String>,
    pub request_timeout_ms: u64,
}

impl From<&WebConfig> for HttpServerConfig {
    fn from(config: &WebConfig) -> Self {
        Self {
            listen: config.listen.clone(),
            cors_origins: config.cors_origins.clone(),
            request_timeout_ms: config.request_timeout_ms,
        }
    }
}

#[derive(Debug)]
pub struct BoundHttpServer {
    listener: TcpListener,
    router: Router,
    local_addr: SocketAddr,
}

impl BoundHttpServer {
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    pub async fn serve_with_shutdown(
        self,
        shutdown: impl Future<Output = ()> + Send + 'static,
    ) -> Result<(), HttpServerError> {
        axum::serve(self.listener, self.router)
            .with_graceful_shutdown(shutdown)
            .await
            .map_err(HttpServerError::Serve)
    }
}

pub async fn bind_http_server(
    config: &HttpServerConfig,
) -> Result<BoundHttpServer, HttpServerError> {
    bind_http_server_with_state(config, ApiState::default()).await
}

pub async fn bind_http_server_with_state(
    config: &HttpServerConfig,
    state: ApiState,
) -> Result<BoundHttpServer, HttpServerError> {
    let listener =
        TcpListener::bind(&config.listen)
            .await
            .map_err(|source| HttpServerError::Bind {
                listen: config.listen.clone(),
                source,
            })?;
    let local_addr = listener.local_addr().map_err(HttpServerError::LocalAddr)?;

    Ok(BoundHttpServer {
        listener,
        router: router_with_config(state, config)?,
        local_addr,
    })
}

pub fn router() -> Router {
    router_with_state(ApiState::default())
}

pub fn router_with_state(state: ApiState) -> Router {
    router_with_state_and_cors(state, CorsLayer::new())
}

fn router_with_config(
    state: ApiState,
    config: &HttpServerConfig,
) -> Result<Router, HttpServerError> {
    Ok(router_with_state_and_cors(state, cors_layer(config)?))
}

fn router_with_state_and_cors(state: ApiState, cors: CorsLayer) -> Router {
    Router::new()
        .merge(health::routes())
        .merge(sql_events::routes())
        .merge(export::routes())
        .merge(connections::routes())
        .merge(statistics::routes())
        .merge(protocols::routes())
        .merge(replay::routes())
        .merge(websocket::routes())
        .fallback(api_not_found)
        .layer(axum::Extension(state))
        .layer(middleware::from_fn(attach_request_id))
        .layer(cors)
}

fn cors_layer(config: &HttpServerConfig) -> Result<CorsLayer, HttpServerError> {
    let origins = config
        .cors_origins
        .iter()
        .filter_map(|origin| {
            let origin = origin.trim();
            (!origin.is_empty()).then_some(origin)
        })
        .map(|origin| {
            HeaderValue::from_str(origin).map_err(|source| HttpServerError::InvalidCorsOrigin {
                origin: origin.to_owned(),
                source,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let request_id_header = header::HeaderName::from_static(REQUEST_ID_HEADER);

    Ok(CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([Method::GET, Method::HEAD, Method::POST, Method::OPTIONS])
        .allow_headers([
            header::ACCEPT,
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            request_id_header.clone(),
        ])
        .expose_headers([request_id_header]))
}

async fn api_not_found(OriginalUri(uri): OriginalUri) -> ApiEndpointError {
    ApiEndpointError::not_found("Route not found", "path", uri.path().to_owned())
}

#[derive(Debug)]
pub enum HttpServerError {
    Bind {
        listen: String,
        source: std::io::Error,
    },
    LocalAddr(std::io::Error),
    InvalidCorsOrigin {
        origin: String,
        source: InvalidHeaderValue,
    },
    Serve(std::io::Error),
}

impl fmt::Display for HttpServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bind { listen, source } => {
                write!(f, "failed to bind HTTP server on {listen}: {source}")
            }
            Self::LocalAddr(source) => {
                write!(f, "failed to read HTTP server local address: {source}")
            }
            Self::InvalidCorsOrigin { origin, source } => {
                write!(f, "invalid CORS origin `{origin}`: {source}")
            }
            Self::Serve(source) => write!(f, "HTTP server failed: {source}"),
        }
    }
}

impl Error for HttpServerError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Bind { source, .. } => Some(source),
            Self::LocalAddr(source) => Some(source),
            Self::InvalidCorsOrigin { source, .. } => Some(source),
            Self::Serve(source) => Some(source),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use axum::{
        body::Body,
        http::{HeaderValue, Request, StatusCode, header},
    };
    use sql_lens_config::WebConfig;
    use tokio::sync::oneshot;
    use tower::ServiceExt;

    use super::{HttpServerConfig, bind_http_server, router_with_config};
    use crate::ApiState;

    #[test]
    fn server_config_uses_web_config_values() {
        let web_config = WebConfig {
            listen: "127.0.0.1:0".to_owned(),
            request_timeout_ms: 15_000,
            ..WebConfig::default()
        };

        let server_config = HttpServerConfig::from(&web_config);

        assert_eq!(server_config.listen, "127.0.0.1:0");
        assert_eq!(server_config.cors_origins, web_config.cors_origins);
        assert_eq!(server_config.request_timeout_ms, 15_000);
    }

    #[tokio::test]
    async fn bind_uses_configured_listen_address() {
        let server = bind_http_server(&HttpServerConfig {
            listen: "127.0.0.1:0".to_owned(),
            cors_origins: Vec::new(),
            request_timeout_ms: 30_000,
        })
        .await
        .expect("server should bind to an ephemeral port");

        assert_eq!(server.local_addr().ip().to_string(), "127.0.0.1");
        assert_ne!(server.local_addr().port(), 0);
    }

    #[tokio::test]
    async fn server_exits_when_shutdown_signal_resolves() {
        let server = bind_http_server(&HttpServerConfig {
            listen: "127.0.0.1:0".to_owned(),
            cors_origins: Vec::new(),
            request_timeout_ms: 30_000,
        })
        .await
        .expect("server should bind to an ephemeral port");
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

        let server_task = tokio::spawn(server.serve_with_shutdown(async move {
            let _ = shutdown_rx.await;
        }));

        shutdown_tx
            .send(())
            .expect("shutdown receiver should be waiting");

        tokio::time::timeout(Duration::from_secs(2), server_task)
            .await
            .expect("server should stop before timeout")
            .expect("server task should not panic")
            .expect("server should stop cleanly");
    }

    #[tokio::test]
    async fn cors_preflight_uses_configured_origin() {
        let config = HttpServerConfig {
            listen: "127.0.0.1:0".to_owned(),
            cors_origins: vec!["http://localhost:5174".to_owned()],
            request_timeout_ms: 30_000,
        };
        let router = router_with_config(ApiState::default(), &config)
            .expect("configured CORS router should build");

        let response = router
            .oneshot(
                Request::builder()
                    .method("OPTIONS")
                    .uri("/api/v1/statistics")
                    .header(header::ORIGIN, "http://localhost:5174")
                    .header(header::ACCESS_CONTROL_REQUEST_METHOD, "GET")
                    .header(header::ACCESS_CONTROL_REQUEST_HEADERS, "content-type")
                    .body(Body::empty())
                    .expect("preflight request should build"),
            )
            .await
            .expect("preflight should be handled");

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN),
            Some(&HeaderValue::from_static("http://localhost:5174"))
        );
        assert!(
            response
                .headers()
                .get(header::ACCESS_CONTROL_ALLOW_METHODS)
                .expect("preflight should include allowed methods")
                .to_str()
                .expect("allowed methods should be text")
                .contains("GET")
        );
        assert!(
            response
                .headers()
                .get(header::ACCESS_CONTROL_ALLOW_HEADERS)
                .expect("preflight should include allowed headers")
                .to_str()
                .expect("allowed headers should be text")
                .contains("content-type")
        );
    }
}
