use axum::{
    Extension, Router,
    body::Bytes,
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::Response,
    routing::get,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sql_lens_core::{CaptureStatus, DurationMillis, ProtocolName, SqlEvent};

use crate::{
    ApiState, SqlEventBroadcaster, SqlEventSubscription, SqlEventSubscriptionError,
    sql_events::SqlEventSummaryResponse,
};

const SQL_EVENT_CREATED_MESSAGE_TYPE: &str = "sql_event.created";
const SUBSCRIPTION_ERROR_MESSAGE_TYPE: &str = "subscription.error";
const SUBSCRIBE_MESSAGE_TYPE: &str = "subscribe";
const WEBSOCKET_MESSAGE_VERSION: u32 = 1;
const INVALID_FILTER_CODE: &str = "INVALID_FILTER";
const INVALID_FILTER_MESSAGE: &str = "invalid subscription filter";
pub const SQL_WS_PATH: &str = "/ws/sql";
const INITIAL_HEARTBEAT_PAYLOAD: &[u8] = b"sql-lens";

pub(crate) fn routes() -> Router {
    Router::new().route(SQL_WS_PATH, get(upgrade_sql_stream))
}

async fn upgrade_sql_stream(
    Extension(state): Extension<ApiState>,
    ws: WebSocketUpgrade,
) -> Response {
    let broadcaster = state.sql_event_broadcaster();
    ws.on_upgrade(move |socket| handle_sql_socket(socket, broadcaster))
}

async fn handle_sql_socket(mut socket: WebSocket, broadcaster: SqlEventBroadcaster) {
    if socket
        .send(Message::Ping(Bytes::from_static(INITIAL_HEARTBEAT_PAYLOAD)))
        .await
        .is_err()
    {
        return;
    }

    while let Some(message) = socket.recv().await {
        match message {
            Ok(Message::Close(_)) | Err(_) => break,
            Ok(Message::Text(text)) => match parse_subscribe_message(text.as_str()) {
                SubscribeMessageAction::Subscribe(filter) => {
                    handle_subscribed_sql_socket(socket, broadcaster.subscribe(), filter).await;
                    break;
                }
                SubscribeMessageAction::Error(error) => {
                    let Ok(message) = subscription_error_message(error) else {
                        break;
                    };

                    if socket.send(Message::Text(message.into())).await.is_err() {
                        break;
                    }
                }
                SubscribeMessageAction::Ignore => {}
            },
            Ok(Message::Binary(_)) | Ok(Message::Ping(_)) | Ok(Message::Pong(_)) => {}
        }
    }
}

async fn handle_subscribed_sql_socket(
    mut socket: WebSocket,
    mut subscription: SqlEventSubscription,
    filter: SqlEventSubscriptionFilter,
) {
    loop {
        tokio::select! {
            message = socket.recv() => {
                match message {
                    Some(Ok(Message::Close(_))) | Some(Err(_)) | None => break,
                    Some(Ok(Message::Text(_)))
                    | Some(Ok(Message::Binary(_)))
                    | Some(Ok(Message::Ping(_)))
                    | Some(Ok(Message::Pong(_))) => {}
                }
            }
            event = subscription.recv() => {
                match event {
                    Ok(event) => {
                        if !filter.matches(&event) {
                            continue;
                        }

                        let Ok(message) = sql_event_created_message(&event) else {
                            break;
                        };

                        if socket.send(Message::Text(message.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(SqlEventSubscriptionError::Lagged { .. }) => {}
                    Err(SqlEventSubscriptionError::Closed) => break,
                }
            }
        }
    }
}

fn parse_subscribe_message(text: &str) -> SubscribeMessageAction {
    let Ok(message) = serde_json::from_str::<SubscribeMessage>(text) else {
        return SubscribeMessageAction::Ignore;
    };

    if message.message_type != SUBSCRIBE_MESSAGE_TYPE
        || message.version != WEBSOCKET_MESSAGE_VERSION
    {
        return SubscribeMessageAction::Ignore;
    }

    match message.filters {
        Some(filters) => match SqlEventSubscriptionFilter::from_value(filters) {
            Ok(filter) => SubscribeMessageAction::Subscribe(filter),
            Err(error) => SubscribeMessageAction::Error(error),
        },
        None => SubscribeMessageAction::Subscribe(SqlEventSubscriptionFilter::default()),
    }
}

fn sql_event_created_message(event: &SqlEvent) -> Result<String, serde_json::Error> {
    serde_json::to_string(&SqlEventCreatedMessage {
        message_type: SQL_EVENT_CREATED_MESSAGE_TYPE,
        version: WEBSOCKET_MESSAGE_VERSION,
        payload: SqlEventSummaryResponse::from(event),
    })
}

fn subscription_error_message(error: SubscriptionFilterError) -> Result<String, serde_json::Error> {
    serde_json::to_string(&SubscriptionErrorMessage {
        message_type: SUBSCRIPTION_ERROR_MESSAGE_TYPE,
        version: WEBSOCKET_MESSAGE_VERSION,
        payload: SubscriptionErrorPayload {
            code: INVALID_FILTER_CODE,
            message: error.message,
            field: error.field,
        },
    })
}

#[derive(Debug, PartialEq, Eq)]
enum SubscribeMessageAction {
    Subscribe(SqlEventSubscriptionFilter),
    Error(SubscriptionFilterError),
    Ignore,
}

#[derive(Debug, Deserialize)]
struct SubscribeMessage {
    #[serde(rename = "type")]
    message_type: String,
    version: u32,
    filters: Option<Value>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct SqlEventSubscriptionFilter {
    target_name: Option<String>,
    protocol: Option<ProtocolName>,
    statuses: Option<Vec<CaptureStatus>>,
    database: Option<String>,
    min_duration: Option<DurationMillis>,
    max_duration: Option<DurationMillis>,
}

impl SqlEventSubscriptionFilter {
    fn from_value(value: Value) -> Result<Self, SubscriptionFilterError> {
        let filters = serde_json::from_value::<SubscribeFilterMessage>(value)
            .map_err(|_| SubscriptionFilterError::new("filters"))?;

        let statuses = filters.status.map(parse_status_filters).transpose()?;
        if let (Some(min), Some(max)) = (filters.min_duration_ms, filters.max_duration_ms)
            && min > max
        {
            return Err(SubscriptionFilterError::new("filters.min_duration_ms"));
        }

        Ok(Self {
            target_name: filters.target_name,
            protocol: filters.protocol.map(ProtocolName),
            statuses,
            database: filters.database,
            min_duration: filters.min_duration_ms.map(DurationMillis),
            max_duration: filters.max_duration_ms.map(DurationMillis),
        })
    }

    fn matches(&self, event: &SqlEvent) -> bool {
        if let Some(target_name) = self.target_name.as_deref()
            && event.target_name.as_deref() != Some(target_name)
        {
            return false;
        }

        if let Some(protocol) = &self.protocol
            && &event.protocol != protocol
        {
            return false;
        }

        if let Some(database) = self.database.as_deref()
            && event.database.as_deref() != Some(database)
        {
            return false;
        }

        if let Some(statuses) = self.statuses.as_deref()
            && !statuses.contains(&event.status)
        {
            return false;
        }

        if let Some(min_duration) = self.min_duration
            && event.duration < min_duration
        {
            return false;
        }

        if let Some(max_duration) = self.max_duration
            && event.duration > max_duration
        {
            return false;
        }

        true
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SubscribeFilterMessage {
    target_name: Option<String>,
    protocol: Option<String>,
    status: Option<Vec<String>>,
    database: Option<String>,
    min_duration_ms: Option<u64>,
    max_duration_ms: Option<u64>,
}

fn parse_status_filters(
    values: Vec<String>,
) -> Result<Vec<CaptureStatus>, SubscriptionFilterError> {
    if values.is_empty() {
        return Err(SubscriptionFilterError::new("filters.status"));
    }

    values
        .into_iter()
        .map(|value| match value.as_str() {
            "ok" => Ok(CaptureStatus::Ok),
            "slow" => Ok(CaptureStatus::Slow),
            "error" => Ok(CaptureStatus::Error),
            "unknown" => Ok(CaptureStatus::Unknown),
            _ => Err(SubscriptionFilterError::new("filters.status")),
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SubscriptionFilterError {
    field: &'static str,
    message: &'static str,
}

impl SubscriptionFilterError {
    fn new(field: &'static str) -> Self {
        Self {
            field,
            message: INVALID_FILTER_MESSAGE,
        }
    }
}

#[derive(Debug, Serialize)]
struct SqlEventCreatedMessage {
    #[serde(rename = "type")]
    message_type: &'static str,
    version: u32,
    payload: SqlEventSummaryResponse,
}

#[derive(Debug, Serialize)]
struct SubscriptionErrorMessage {
    #[serde(rename = "type")]
    message_type: &'static str,
    version: u32,
    payload: SubscriptionErrorPayload,
}

#[derive(Debug, Serialize)]
struct SubscriptionErrorPayload {
    code: &'static str,
    message: &'static str,
    field: &'static str,
}

#[cfg(test)]
mod tests {
    use std::{net::SocketAddr, time::Duration};

    use super::{SubscribeMessageAction, SubscriptionFilterError, parse_subscribe_message};
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use futures_util::{SinkExt, StreamExt};
    use serde_json::Value;
    use sql_lens_core::{CaptureStatus, DurationMillis, ProtocolName};
    use tokio::{net::TcpListener, sync::oneshot, task::JoinHandle};
    use tokio_tungstenite::{
        MaybeTlsStream, WebSocketStream, connect_async, tungstenite::Message as ClientMessage,
    };
    use tower::ServiceExt;

    use crate::{
        ApiState, HttpServerConfig, REQUEST_ID_HEADER, SQL_WS_PATH, bind_http_server, router,
        router_with_state, test_support::test_event, websocket::INITIAL_HEARTBEAT_PAYLOAD,
    };

    type TestClient = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

    #[test]
    fn subscription_filter_matches_protocol_status_database_and_duration() {
        let filter = match parse_subscribe_message(
            r#"{
                "type":"subscribe",
                "version":1,
                "filters":{
                    "target_name":"mysql-local",
                    "protocol":"mysql",
                    "status":["ok","slow"],
                    "database":"app",
                    "min_duration_ms":3,
                    "max_duration_ms":10
                }
            }"#,
        ) {
            SubscribeMessageAction::Subscribe(filter) => filter,
            other => panic!("expected valid subscription filter, got {other:?}"),
        };

        assert!(filter.matches(&test_event("evt_match")));
        assert!(!filter.matches(&event_with_target_name("evt_target", "starrocks-local")));
        assert!(!filter.matches(&event_with_protocol("evt_protocol", "postgresql")));
        assert!(!filter.matches(&event_with_status("evt_status", CaptureStatus::Error)));
        assert!(!filter.matches(&event_with_database("evt_database", "analytics")));
        assert!(!filter.matches(&event_with_duration("evt_duration", 11)));
    }

    #[test]
    fn invalid_subscription_filters_return_filter_errors() {
        assert_eq!(
            parse_subscribe_message(
                r#"{"type":"subscribe","version":1,"filters":{"status":["bad"]}}"#
            ),
            SubscribeMessageAction::Error(SubscriptionFilterError::new("filters.status"))
        );
        assert_eq!(
            parse_subscribe_message(r#"{"type":"subscribe","version":1,"filters":{"status":[]}}"#),
            SubscribeMessageAction::Error(SubscriptionFilterError::new("filters.status"))
        );
        assert_eq!(
            parse_subscribe_message(
                r#"{"type":"subscribe","version":1,"filters":{"min_duration_ms":20,"max_duration_ms":10}}"#
            ),
            SubscribeMessageAction::Error(SubscriptionFilterError::new("filters.min_duration_ms"))
        );
        assert_eq!(
            parse_subscribe_message(
                r#"{"type":"subscribe","version":1,"filters":{"unsupported":"value"}}"#
            ),
            SubscribeMessageAction::Error(SubscriptionFilterError::new("filters"))
        );
    }

    #[tokio::test]
    async fn websocket_upgrade_sends_initial_ping_and_closes_cleanly() {
        let server = bind_http_server(&HttpServerConfig {
            listen: "127.0.0.1:0".to_owned(),
            request_timeout_ms: 30_000,
        })
        .await
        .expect("server should bind to an ephemeral port");
        let addr = server.local_addr();
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        let server_task = tokio::spawn(server.serve_with_shutdown(async move {
            let _ = shutdown_rx.await;
        }));

        let (mut client, response) = connect_async(format!("ws://{addr}{SQL_WS_PATH}"))
            .await
            .expect("websocket client should connect");

        assert_eq!(response.status().as_u16(), StatusCode::SWITCHING_PROTOCOLS);
        assert!(
            response.headers().contains_key(REQUEST_ID_HEADER),
            "upgrade response should include request ID"
        );

        expect_initial_ping(&mut client).await;

        client
            .close(None)
            .await
            .expect("client close should be sent");
        let _ = shutdown_tx.send(());

        tokio::time::timeout(Duration::from_secs(2), server_task)
            .await
            .expect("server should stop before timeout")
            .expect("server task should not panic")
            .expect("server should stop cleanly");
    }

    #[tokio::test]
    async fn websocket_waits_for_subscribe_before_sending_events() {
        let state = ApiState::default();
        let broadcaster = state.sql_event_broadcaster();
        let (addr, shutdown_tx, server_task) = spawn_test_server(state).await;
        let (mut client, _) = connect_async(format!("ws://{addr}{SQL_WS_PATH}"))
            .await
            .expect("websocket client should connect");
        expect_initial_ping(&mut client).await;

        assert_eq!(
            broadcaster.publish(test_event("evt_before_subscribe")),
            crate::SqlEventBroadcastOutcome::NoSubscribers
        );
        assert_no_message(&mut client).await;

        client
            .send(ClientMessage::text(r#"{"type":"subscribe","version":1}"#))
            .await
            .expect("subscribe message should send");
        wait_for_subscriber(&broadcaster).await;

        assert_eq!(
            broadcaster.publish(test_event("evt_after_subscribe")),
            crate::SqlEventBroadcastOutcome::Delivered {
                subscriber_count: 1
            }
        );
        let message = next_message(&mut client).await;

        assert_sql_event_created_message(message, "evt_after_subscribe");
        client.close(None).await.expect("client should close");
        stop_test_server(shutdown_tx, server_task).await;
    }

    #[tokio::test]
    async fn invalid_subscribe_messages_are_ignored_until_valid_subscribe() {
        let state = ApiState::default();
        let broadcaster = state.sql_event_broadcaster();
        let (addr, shutdown_tx, server_task) = spawn_test_server(state).await;
        let (mut client, _) = connect_async(format!("ws://{addr}{SQL_WS_PATH}"))
            .await
            .expect("websocket client should connect");
        expect_initial_ping(&mut client).await;

        client
            .send(ClientMessage::text("not json"))
            .await
            .expect("invalid JSON should send");
        client
            .send(ClientMessage::text(r#"{"type":"subscribe","version":99}"#))
            .await
            .expect("wrong version should send");
        assert_no_message(&mut client).await;

        client
            .send(ClientMessage::text(r#"{"type":"subscribe","version":1}"#))
            .await
            .expect("valid subscribe should send after invalid messages");
        wait_for_subscriber(&broadcaster).await;

        assert_eq!(
            broadcaster.publish(test_event("evt_after_invalid_messages")),
            crate::SqlEventBroadcastOutcome::Delivered {
                subscriber_count: 1
            }
        );
        let message = next_message(&mut client).await;

        assert_sql_event_created_message(message, "evt_after_invalid_messages");
        client.close(None).await.expect("client should close");
        stop_test_server(shutdown_tx, server_task).await;
    }

    #[tokio::test]
    async fn websocket_protocol_filter_sends_matching_events_only() {
        let state = ApiState::default();
        let broadcaster = state.sql_event_broadcaster();
        let (addr, shutdown_tx, server_task) = spawn_test_server(state).await;
        let (mut client, _) = connect_async(format!("ws://{addr}{SQL_WS_PATH}"))
            .await
            .expect("websocket client should connect");
        expect_initial_ping(&mut client).await;

        send_subscribe(
            &mut client,
            r#"{"type":"subscribe","version":1,"filters":{"protocol":"mysql"}}"#,
        )
        .await;
        wait_for_subscriber(&broadcaster).await;

        broadcaster.publish(event_with_protocol("evt_postgres", "postgresql"));
        assert_no_message(&mut client).await;

        broadcaster.publish(event_with_protocol("evt_mysql", "mysql"));
        assert_sql_event_created_message(next_message(&mut client).await, "evt_mysql");
        client.close(None).await.expect("client should close");
        stop_test_server(shutdown_tx, server_task).await;
    }

    #[tokio::test]
    async fn websocket_status_filter_supports_multiple_statuses() {
        let state = ApiState::default();
        let broadcaster = state.sql_event_broadcaster();
        let (addr, shutdown_tx, server_task) = spawn_test_server(state).await;
        let (mut client, _) = connect_async(format!("ws://{addr}{SQL_WS_PATH}"))
            .await
            .expect("websocket client should connect");
        expect_initial_ping(&mut client).await;

        send_subscribe(
            &mut client,
            r#"{"type":"subscribe","version":1,"filters":{"status":["error","slow"]}}"#,
        )
        .await;
        wait_for_subscriber(&broadcaster).await;

        broadcaster.publish(event_with_status("evt_ok", CaptureStatus::Ok));
        assert_no_message(&mut client).await;

        broadcaster.publish(event_with_status("evt_error", CaptureStatus::Error));
        assert_sql_event_created_message(next_message(&mut client).await, "evt_error");

        broadcaster.publish(event_with_status("evt_slow", CaptureStatus::Slow));
        assert_sql_event_created_message(next_message(&mut client).await, "evt_slow");
        client.close(None).await.expect("client should close");
        stop_test_server(shutdown_tx, server_task).await;
    }

    #[tokio::test]
    async fn websocket_database_filter_sends_matching_events_only() {
        let state = ApiState::default();
        let broadcaster = state.sql_event_broadcaster();
        let (addr, shutdown_tx, server_task) = spawn_test_server(state).await;
        let (mut client, _) = connect_async(format!("ws://{addr}{SQL_WS_PATH}"))
            .await
            .expect("websocket client should connect");
        expect_initial_ping(&mut client).await;

        send_subscribe(
            &mut client,
            r#"{"type":"subscribe","version":1,"filters":{"database":"app"}}"#,
        )
        .await;
        wait_for_subscriber(&broadcaster).await;

        broadcaster.publish(event_with_database("evt_other_db", "analytics"));
        assert_no_message(&mut client).await;

        broadcaster.publish(event_with_database("evt_app_db", "app"));
        assert_sql_event_created_message(next_message(&mut client).await, "evt_app_db");
        client.close(None).await.expect("client should close");
        stop_test_server(shutdown_tx, server_task).await;
    }

    #[tokio::test]
    async fn websocket_duration_filter_sends_matching_events_only() {
        let state = ApiState::default();
        let broadcaster = state.sql_event_broadcaster();
        let (addr, shutdown_tx, server_task) = spawn_test_server(state).await;
        let (mut client, _) = connect_async(format!("ws://{addr}{SQL_WS_PATH}"))
            .await
            .expect("websocket client should connect");
        expect_initial_ping(&mut client).await;

        send_subscribe(
            &mut client,
            r#"{"type":"subscribe","version":1,"filters":{"min_duration_ms":10,"max_duration_ms":20}}"#,
        )
        .await;
        wait_for_subscriber(&broadcaster).await;

        broadcaster.publish(event_with_duration("evt_fast", 9));
        broadcaster.publish(event_with_duration("evt_slow", 21));
        assert_no_message(&mut client).await;

        broadcaster.publish(event_with_duration("evt_in_range", 10));
        assert_sql_event_created_message(next_message(&mut client).await, "evt_in_range");
        client.close(None).await.expect("client should close");
        stop_test_server(shutdown_tx, server_task).await;
    }

    #[tokio::test]
    async fn invalid_filters_return_subscription_error_and_allow_later_subscribe() {
        let state = ApiState::default();
        let broadcaster = state.sql_event_broadcaster();
        let (addr, shutdown_tx, server_task) = spawn_test_server(state).await;
        let (mut client, _) = connect_async(format!("ws://{addr}{SQL_WS_PATH}"))
            .await
            .expect("websocket client should connect");
        expect_initial_ping(&mut client).await;

        client
            .send(ClientMessage::text(
                r#"{"type":"subscribe","version":1,"filters":{"status":["bad"]}}"#,
            ))
            .await
            .expect("invalid filter subscribe should send");
        assert_subscription_error(next_message(&mut client).await, "filters.status");

        send_subscribe(
            &mut client,
            r#"{"type":"subscribe","version":1,"filters":{"status":["ok"]}}"#,
        )
        .await;
        wait_for_subscriber(&broadcaster).await;

        broadcaster.publish(event_with_status(
            "evt_after_filter_error",
            CaptureStatus::Ok,
        ));
        assert_sql_event_created_message(next_message(&mut client).await, "evt_after_filter_error");
        client.close(None).await.expect("client should close");
        stop_test_server(shutdown_tx, server_task).await;
    }

    #[tokio::test]
    async fn plain_http_request_to_websocket_path_is_rejected() {
        let response = router()
            .oneshot(
                Request::builder()
                    .uri(SQL_WS_PATH)
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should be handled");

        assert_ne!(response.status(), StatusCode::OK);
        assert!(
            response.headers().contains_key(REQUEST_ID_HEADER),
            "websocket rejection should include request ID"
        );
    }

    async fn spawn_test_server(
        state: ApiState,
    ) -> (
        SocketAddr,
        oneshot::Sender<()>,
        JoinHandle<Result<(), std::io::Error>>,
    ) {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("test server should bind");
        let addr = listener
            .local_addr()
            .expect("test server local address should be available");
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        let server_task = tokio::spawn(async move {
            axum::serve(listener, router_with_state(state))
                .with_graceful_shutdown(async move {
                    let _ = shutdown_rx.await;
                })
                .await
        });

        (addr, shutdown_tx, server_task)
    }

    async fn stop_test_server(
        shutdown_tx: oneshot::Sender<()>,
        server_task: JoinHandle<Result<(), std::io::Error>>,
    ) {
        let _ = shutdown_tx.send(());
        tokio::time::timeout(Duration::from_secs(2), server_task)
            .await
            .expect("server should stop before timeout")
            .expect("server task should not panic")
            .expect("server should stop cleanly");
    }

    async fn wait_for_subscriber(broadcaster: &crate::SqlEventBroadcaster) {
        for _ in 0..20 {
            if broadcaster.subscriber_count() > 0 {
                return;
            }

            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        panic!("websocket subscription was not registered before timeout");
    }

    async fn send_subscribe(client: &mut TestClient, message: &str) {
        client
            .send(ClientMessage::text(message))
            .await
            .expect("subscribe message should send");
    }

    async fn expect_initial_ping(client: &mut TestClient) {
        match next_message(client).await {
            ClientMessage::Ping(payload) => assert_eq!(payload.as_ref(), INITIAL_HEARTBEAT_PAYLOAD),
            other => panic!("expected initial ping heartbeat, got {other:?}"),
        }
    }

    async fn next_message(client: &mut TestClient) -> ClientMessage {
        tokio::time::timeout(Duration::from_secs(2), client.next())
            .await
            .expect("server should send a websocket message")
            .expect("websocket stream should still be open")
            .expect("websocket message should be valid")
    }

    async fn assert_no_message(client: &mut TestClient) {
        assert!(
            tokio::time::timeout(Duration::from_millis(100), client.next())
                .await
                .is_err(),
            "websocket should not receive a message"
        );
    }

    fn assert_sql_event_created_message(message: ClientMessage, expected_id: &str) {
        let ClientMessage::Text(text) = message else {
            panic!("expected sql_event.created text message, got {message:?}");
        };
        let payload: Value =
            serde_json::from_str(text.as_str()).expect("websocket text should be JSON");

        assert_eq!(payload["type"], "sql_event.created");
        assert_eq!(payload["version"], 1);
        assert_eq!(payload["payload"]["id"], expected_id);
        assert_eq!(payload["payload"]["target_name"], "mysql-local");
    }

    fn assert_subscription_error(message: ClientMessage, expected_field: &str) {
        let ClientMessage::Text(text) = message else {
            panic!("expected subscription.error text message, got {message:?}");
        };
        let payload: Value =
            serde_json::from_str(text.as_str()).expect("websocket text should be JSON");

        assert_eq!(payload["type"], "subscription.error");
        assert_eq!(payload["version"], 1);
        assert_eq!(payload["payload"]["code"], "INVALID_FILTER");
        assert_eq!(payload["payload"]["message"], "invalid subscription filter");
        assert_eq!(payload["payload"]["field"], expected_field);
    }

    fn event_with_protocol(id: &str, protocol: &str) -> sql_lens_core::SqlEvent {
        let mut event = test_event(id);
        event.protocol = ProtocolName(protocol.to_owned());
        event
    }

    fn event_with_target_name(id: &str, target_name: &str) -> sql_lens_core::SqlEvent {
        let mut event = test_event(id);
        event.target_name = Some(target_name.to_owned());
        event
    }

    fn event_with_status(id: &str, status: CaptureStatus) -> sql_lens_core::SqlEvent {
        let mut event = test_event(id);
        event.status = status;
        event
    }

    fn event_with_database(id: &str, database: &str) -> sql_lens_core::SqlEvent {
        let mut event = test_event(id);
        event.database = Some(database.to_owned());
        event
    }

    fn event_with_duration(id: &str, duration_ms: u64) -> sql_lens_core::SqlEvent {
        let mut event = test_event(id);
        event.duration = DurationMillis(duration_ms);
        event
    }
}
