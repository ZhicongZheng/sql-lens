use axum::{
    Router,
    body::Bytes,
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::Response,
    routing::get,
};

pub const SQL_WS_PATH: &str = "/ws/sql";
const INITIAL_HEARTBEAT_PAYLOAD: &[u8] = b"sql-lens";

pub(crate) fn routes() -> Router {
    Router::new().route(SQL_WS_PATH, get(upgrade_sql_stream))
}

async fn upgrade_sql_stream(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(handle_sql_socket)
}

async fn handle_sql_socket(mut socket: WebSocket) {
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
            Ok(Message::Text(_))
            | Ok(Message::Binary(_))
            | Ok(Message::Ping(_))
            | Ok(Message::Pong(_)) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use futures_util::StreamExt;
    use tokio::sync::oneshot;
    use tokio_tungstenite::{connect_async, tungstenite::Message as ClientMessage};
    use tower::ServiceExt;

    use crate::{
        HttpServerConfig, REQUEST_ID_HEADER, SQL_WS_PATH, bind_http_server, router,
        websocket::INITIAL_HEARTBEAT_PAYLOAD,
    };

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

        let message = tokio::time::timeout(Duration::from_secs(2), client.next())
            .await
            .expect("server should send initial heartbeat")
            .expect("websocket stream should still be open")
            .expect("heartbeat message should be valid");

        match message {
            ClientMessage::Ping(payload) => assert_eq!(payload.as_ref(), INITIAL_HEARTBEAT_PAYLOAD),
            other => panic!("expected initial ping heartbeat, got {other:?}"),
        }

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
}
