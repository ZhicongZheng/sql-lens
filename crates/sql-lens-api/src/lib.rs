//! REST and WebSocket API surface for SQL Lens.

mod api_state;
mod health;
mod request_id;
mod server;
mod sql_events;

pub use api_state::{ApiState, DEFAULT_EVENT_STORE_CAPACITY};
pub use health::{HEALTH_PATH, HealthResponse, HealthState};
pub use request_id::{REQUEST_ID_HEADER, RequestId};
pub use server::{
    BoundHttpServer, HttpServerConfig, HttpServerError, bind_http_server, router, router_with_state,
};
pub use sql_events::{
    ErrorSummaryResponse, MetadataValueResponse, ProtocolMetadataResponse, QueryTimingResponse,
    RowsSummaryResponse, SQL_EVENT_DETAIL_PATH, SQL_EVENTS_PATH, SqlEventDetailResponse,
    SqlEventListResponse, SqlEventSummaryResponse, SqlParameterResponse,
    SqlParameterValueDataResponse, SqlParameterValueResponse,
};
