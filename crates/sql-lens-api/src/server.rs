use std::{error::Error, fmt, future::Future, net::SocketAddr};

use axum::{Router, extract::OriginalUri, middleware};
use sql_lens_config::WebConfig;
use tokio::net::TcpListener;

use crate::{
    ApiState, api_error::ApiEndpointError, connections, health, protocols,
    request_id::attach_request_id, sql_events, statistics, websocket,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpServerConfig {
    pub listen: String,
    pub request_timeout_ms: u64,
}

impl From<&WebConfig> for HttpServerConfig {
    fn from(config: &WebConfig) -> Self {
        Self {
            listen: config.listen.clone(),
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
        router: router_with_state(state),
        local_addr,
    })
}

pub fn router() -> Router {
    router_with_state(ApiState::default())
}

pub fn router_with_state(state: ApiState) -> Router {
    Router::new()
        .merge(health::routes())
        .merge(sql_events::routes())
        .merge(connections::routes())
        .merge(statistics::routes())
        .merge(protocols::routes())
        .merge(websocket::routes())
        .fallback(api_not_found)
        .layer(axum::Extension(state))
        .layer(middleware::from_fn(attach_request_id))
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
            Self::Serve(source) => write!(f, "HTTP server failed: {source}"),
        }
    }
}

impl Error for HttpServerError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Bind { source, .. } => Some(source),
            Self::LocalAddr(source) => Some(source),
            Self::Serve(source) => Some(source),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use sql_lens_config::WebConfig;
    use tokio::sync::oneshot;

    use super::{HttpServerConfig, bind_http_server};

    #[test]
    fn server_config_uses_web_config_values() {
        let web_config = WebConfig {
            listen: "127.0.0.1:0".to_owned(),
            request_timeout_ms: 15_000,
            ..WebConfig::default()
        };

        let server_config = HttpServerConfig::from(&web_config);

        assert_eq!(server_config.listen, "127.0.0.1:0");
        assert_eq!(server_config.request_timeout_ms, 15_000);
    }

    #[tokio::test]
    async fn bind_uses_configured_listen_address() {
        let server = bind_http_server(&HttpServerConfig {
            listen: "127.0.0.1:0".to_owned(),
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
}
