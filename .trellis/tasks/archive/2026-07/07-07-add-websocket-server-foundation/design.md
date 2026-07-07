# Add WebSocket server foundation design

## Boundary

Implement in `crates/sql-lens-api`.

This task creates the WebSocket upgrade and socket lifecycle foundation for `GET /ws/sql`. It must not implement SQL event streaming, subscription filters, storage fan-out, statistics streaming, authentication, replay, or frontend code.

## Current State

`sql-lens-api` currently has:

- REST route modules under `health`, `sql_events`, `connections`, `statistics`, and `protocols`.
- `server::router_with_state` merges route modules and applies request ID middleware.
- Standard API error fallback for unmatched REST routes.
- `axum = "0.8"` without the `ws` feature.

There is no WebSocket module yet.

## Dependency Change

Enable Axum's WebSocket feature in `crates/sql-lens-api/Cargo.toml`:

```toml
axum = { version = "0.8", features = ["ws"] }
```

No additional runtime crate should be added for the first foundation. Existing `tokio` features should be enough for basic async socket handling and tests; add narrower features only if the compiler requires them.

## API Contract

Endpoint:

```text
GET /ws/sql
```

Public constants:

```rust
pub const SQL_WS_PATH: &str = "/ws/sql";
```

This task does not define the subscription JSON payload contract in code. Later SQL WebSocket subscription work owns message schemas such as:

```json
{
  "type": "subscribe",
  "version": 1,
  "filters": {
    "protocol": "mysql",
    "status": ["ok", "error", "slow"]
  }
}
```

## Module Shape

Add `crates/sql-lens-api/src/websocket.rs`.

Route module:

```rust
pub(crate) fn routes() -> Router {
    Router::new().route(SQL_WS_PATH, get(upgrade_sql_stream))
}
```

Handler:

```rust
async fn upgrade_sql_stream(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(handle_sql_socket)
}
```

Socket lifecycle:

```rust
async fn handle_sql_socket(mut socket: WebSocket) {
    let _ = socket.send(Message::Ping(Bytes::from_static(b"sql-lens"))).await;

    while let Some(message) = socket.recv().await {
        match message {
            Ok(Message::Close(_)) => break,
            Ok(Message::Text(_)) => {}
            Ok(Message::Binary(_)) => {}
            Ok(Message::Ping(_)) => {}
            Ok(Message::Pong(_)) => {}
            Err(_) => break,
        }
    }
}
```

Keep this intentionally small. The first implementation should ignore client text/binary messages because subscription parsing belongs to Issue 035.

## Router Integration

Update `server::router_with_state`:

```rust
Router::new()
    .merge(...)
    .merge(websocket::routes())
    .fallback(api_not_found)
    .layer(Extension(state))
    .layer(middleware::from_fn(attach_request_id))
```

The WebSocket route should be registered before fallback.

The request ID middleware still applies to the HTTP upgrade response header.

## Error Behavior

Valid WebSocket upgrade:

- Returns switching-protocols response.
- Upgrade task handles socket lifecycle.

Plain HTTP `GET /ws/sql` without WebSocket upgrade headers:

- Let Axum's `WebSocketUpgrade` extractor reject the request.
- Do not force this into the SQL Lens JSON API error envelope in this task, because WebSocket upgrade extractor errors are part of the protocol handshake path and not ordinary REST endpoint errors.

Unknown paths:

- Existing API fallback still returns standardized `NOT_FOUND` envelopes.

## Heartbeat

The foundation heartbeat is intentionally minimal:

- Send one initial `Message::Ping` after upgrade.
- Treat send failure as a clean early disconnect.
- Read and ignore `Pong` frames.

Do not add periodic heartbeat intervals until a later task needs long-lived connection liveness semantics. Periodic heartbeat introduces timer policy, timeout policy, and backpressure choices that are not necessary for the first upgrade foundation.

## Tests

Preferred tests:

- Use a real bound local HTTP server from `bind_http_server` or `axum::serve` with `router()`.
- Use a WebSocket client from an existing dependency only if already available or unavoidable.

If no WebSocket client dependency exists, add a minimal dev-dependency with clear scope. Candidate:

```toml
tokio-tungstenite = "0.27"
```

Test cases:

- Valid WebSocket client connects to `/ws/sql`.
- Client receives initial ping or can complete a ping/pong interaction.
- Client close completes without the server task panicking.
- Plain `GET /ws/sql` without upgrade does not return HTTP 200.
- Existing REST endpoint tests pass.

## Compatibility

This is additive:

- No existing REST response schemas change.
- No existing API state changes are required.
- No SQL capture pipeline changes are required.

## Rollback

If WebSocket client tests require too much dependency churn, keep endpoint unit coverage to upgrade rejection and route registration, then add a dedicated live integration test in a follow-up. Do not block the foundation route on SQL event streaming.
