//! REST and WebSocket API surface for SQL Lens.

mod health;
mod request_id;
mod server;

pub use health::{HEALTH_PATH, HealthResponse, HealthState};
pub use request_id::{REQUEST_ID_HEADER, RequestId};
pub use server::{BoundHttpServer, HttpServerConfig, HttpServerError, bind_http_server, router};
