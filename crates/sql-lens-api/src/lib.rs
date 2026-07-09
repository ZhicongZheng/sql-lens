//! REST and WebSocket API surface for SQL Lens.

mod api_error;
mod api_state;
mod connections;
mod event_reader;
mod export;
mod health;
mod live_sql_events;
mod openapi;
mod protocols;
mod replay;
mod request_id;
mod server;
mod sql_events;
mod statistics;
#[cfg(test)]
mod test_support;
mod websocket;

pub use api_error::{ApiErrorBody, ApiErrorEnvelope};
pub use api_state::{ApiState, DEFAULT_CONNECTION_STORE_CAPACITY, DEFAULT_EVENT_STORE_CAPACITY};
pub use connections::{
    CONNECTION_DETAIL_PATH, CONNECTIONS_PATH, ConnectionListResponse, ConnectionResponse,
};
pub use export::{MAX_EXPORT_LIMIT, SQL_EVENTS_EXPORT_PATH};
pub use health::{HEALTH_PATH, HealthResponse, HealthState};
pub use live_sql_events::{
    DEFAULT_SQL_EVENT_BROADCAST_CAPACITY, SqlEventBroadcastOutcome, SqlEventBroadcastStats,
    SqlEventBroadcaster, SqlEventSubscription, SqlEventSubscriptionError,
};
pub use openapi::{openapi, openapi_yaml};
pub use protocols::{PROTOCOLS_PATH, ProtocolListResponse, ProtocolResponse};
pub use replay::{REPLAY_PREVIEW_PATH, ReplayPreviewRequest, ReplayPreviewResponse};
pub use request_id::{REQUEST_ID_HEADER, RequestId};
pub use server::{
    BoundHttpServer, HttpServerConfig, HttpServerError, bind_http_server,
    bind_http_server_with_state, router, router_with_state,
};
pub use sql_events::{
    ErrorSummaryResponse, MetadataValueResponse, ProtocolMetadataResponse, QueryTimingResponse,
    RowsSummaryResponse, SQL_EVENT_DETAIL_PATH, SQL_EVENTS_PATH, SqlEventDetailResponse,
    SqlEventListResponse, SqlEventSummaryResponse, SqlParameterResponse,
    SqlParameterValueDataResponse, SqlParameterValueResponse,
};
pub use statistics::{LatencyPercentilesResponse, STATISTICS_PATH, StatisticsResponse};
pub use websocket::SQL_WS_PATH;
