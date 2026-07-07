use axum::{
    Extension, Router,
    body::Bytes,
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::Response,
    routing::get,
};
use serde::{Deserialize, Serialize};
use sql_lens_core::SqlEvent;

use crate::{
    ApiState, SqlEventBroadcaster, SqlEventSubscription, SqlEventSubscriptionError,
    sql_events::SqlEventSummaryResponse,
};

const SQL_EVENT_CREATED_MESSAGE_TYPE: &str = "sql_event.created";
const SUBSCRIBE_MESSAGE_TYPE: &str = "subscribe";
const WEBSOCKET_MESSAGE_VERSION: u32 = 1;
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
            Ok(Message::Text(text)) if is_valid_subscribe_message(text.as_str()) => {
                handle_subscribed_sql_socket(socket, broadcaster.subscribe()).await;
                break;
            }
            Ok(Message::Text(_))
            | Ok(Message::Binary(_))
            | Ok(Message::Ping(_))
            | Ok(Message::Pong(_)) => {}
        }
    }
}

async fn handle_subscribed_sql_socket(
    mut socket: WebSocket,
    mut subscription: SqlEventSubscription,
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

fn is_valid_subscribe_message(text: &str) -> bool {
    serde_json::from_str::<SubscribeMessage>(text).is_ok_and(|message| {
        message.message_type == SUBSCRIBE_MESSAGE_TYPE
            && message.version == WEBSOCKET_MESSAGE_VERSION
    })
}

fn sql_event_created_message(event: &SqlEvent) -> Result<String, serde_json::Error> {
    serde_json::to_string(&SqlEventCreatedMessage {
        message_type: SQL_EVENT_CREATED_MESSAGE_TYPE,
        version: WEBSOCKET_MESSAGE_VERSION,
        payload: SqlEventSummaryResponse::from(event),
    })
}

#[derive(Debug, Deserialize)]
struct SubscribeMessage {
    #[serde(rename = "type")]
    message_type: String,
    version: u32,
}

#[derive(Debug, Serialize)]
struct SqlEventCreatedMessage {
    #[serde(rename = "type")]
    message_type: &'static str,
    version: u32,
    payload: SqlEventSummaryResponse,
}

#[cfg(test)]
mod tests {
    use std::{net::SocketAddr, time::Duration};

    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use futures_util::{SinkExt, StreamExt};
    use serde_json::Value;
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
            "websocket should not receive a message before valid subscribe"
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
        assert_eq!(payload["payload"]["protocol"], "mysql");
        assert_eq!(payload["payload"]["status"], "ok");
        assert_eq!(payload["payload"]["duration_ms"], 3);
    }
}
