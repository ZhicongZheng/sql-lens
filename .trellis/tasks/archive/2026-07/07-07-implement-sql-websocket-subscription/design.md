# Implement SQL WebSocket subscription design

## Boundary

Implement in `crates/sql-lens-api`.

This task connects the existing `/ws/sql` WebSocket foundation to an API-owned live SQL event broadcaster. It does not wire the proxy runtime capture receiver into the API server; future `sql-lens-app` runtime composition owns that service wiring.

## Current State

- `sql-lens-capture` provides a single-consumer `mpsc` capture channel for `SqlEvent` values.
- `sql-lens-api::ApiState` owns REST-facing state for ring buffer storage, connection storage, and live statistics.
- `sql-lens-api::websocket` accepts upgrades, sends an initial ping heartbeat, and reads until close/error.
- `API.md` documents `subscribe` client messages and `sql_event.created` server messages.
- The product decision for this task is explicit: a socket must send `{"type":"subscribe","version":1}` before receiving SQL events.

## Architecture

Add an API-local broadcast boundary:

```rust
pub struct SqlEventBroadcaster;
pub struct SqlEventSubscription;
pub struct SqlEventBroadcastStats;

impl SqlEventBroadcaster {
    pub fn new(capacity: NonZeroUsize) -> Self;
    pub fn publish(&self, event: SqlEvent) -> SqlEventBroadcastOutcome;
    pub fn subscribe(&self) -> SqlEventSubscription;
    pub fn subscriber_count(&self) -> usize;
    pub fn stats(&self) -> SqlEventBroadcastStats;
}
```

Recommended implementation:

- Use `tokio::sync::broadcast::Sender<SqlEvent>`.
- Store the broadcaster in `ApiState`.
- Expose `ApiState::sql_event_broadcaster()` so tests and future runtime composition can publish live events.
- Keep `SqlEventBroadcaster` in the API crate because it is an API/WebSocket fan-out concern, not a capture hot-path concern.

## Data Flow

```text
future runtime fan-out
        |
        v
ApiState.sql_event_broadcaster.publish(SqlEvent)
        |
        v
tokio broadcast channel
        |
        v
/ws/sql subscribed sockets
        |
        v
Text JSON:
{
  "type": "sql_event.created",
  "version": 1,
  "payload": SqlEventSummaryResponse
}
```

For this task, tests can publish directly through `ApiState::sql_event_broadcaster()` after opening a WebSocket connection. This proves the API broadcast boundary and WebSocket delivery without adding app/runtime orchestration.

## WebSocket Protocol

Client subscription message:

```json
{
  "type": "subscribe",
  "version": 1
}
```

Notes:

- Filters are intentionally ignored in this task even if the client includes a `filters` object. Issue 036 owns filter parsing and matching.
- Binary messages do not subscribe.
- A socket starts in `WaitingForSubscribe`.
- After a valid subscribe message, it transitions to `Subscribed` and begins forwarding broadcast events.

Server event:

```json
{
  "type": "sql_event.created",
  "version": 1,
  "payload": { "...": "SqlEventSummaryResponse fields" }
}
```

The payload should use `SqlEventSummaryResponse::from(&event)` to stay aligned with REST list response conventions.

## Dependencies

Runtime JSON serialization requires moving `serde_json` from dev-only to normal `sql-lens-api` dependency:

```toml
serde_json = "1.0"
```

No new async/runtime dependencies are needed; `tokio` already has `sync`, and WebSocket tests already use `tokio-tungstenite` plus `futures-util`.

## Error And Backpressure Behavior

Broadcast behavior:

- If there are no active subscribers, publishing returns `NoSubscribers` and should not be treated as a hard API/runtime error.
- If a subscriber lags, that subscriber may miss events and should continue reading newer messages. Lag handling should not block publishers or other subscribers.
- Serialization failure is unlikely for owned DTOs but should end that socket lifecycle cleanly if it occurs.

Invalid subscription behavior:

- Ignore malformed, unsupported, or wrong-version text messages while waiting for a valid subscribe.
- Keep the socket open and continue waiting.
- Add a later WebSocket error protocol task if the UI needs user-visible subscription errors.

## Compatibility

This is additive:

- Existing REST schemas do not change.
- Existing `/ws/sql` upgrade behavior remains valid.
- Existing initial ping heartbeat remains.
- Existing `ApiState::default()` and storage-oriented constructors should keep working by installing a default broadcaster.

## Rollback

If WebSocket broadcast tests become flaky, keep the pure broadcaster unit tests and one endpoint smoke test, then split live socket delivery into a smaller follow-up. Do not mix this with capture runtime wiring.
