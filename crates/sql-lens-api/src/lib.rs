//! REST and WebSocket API surface for SQL Lens.

mod request_id;
mod server;

pub use request_id::{REQUEST_ID_HEADER, RequestId};
pub use server::{BoundHttpServer, HttpServerConfig, HttpServerError, bind_http_server, router};
