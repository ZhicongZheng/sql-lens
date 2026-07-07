# Implement SQL WebSocket subscription

## Goal

Implement Issue 035: broadcast newly captured SQL events to `/ws/sql` WebSocket subscribers so the future dashboard can receive live SQL timeline updates without polling.

## Background

- Issue 018 is complete: `sql-lens-capture` exposes a bounded `mpsc` capture pipeline for `SqlEvent` values.
- Issue 034 is complete: `sql-lens-api` registers `GET /ws/sql`, accepts WebSocket upgrades, sends an initial ping heartbeat, and handles close/disconnect cleanly.
- `API.md` documents server messages with `type`, `version`, and `payload`.
- `API.md` documents the live SQL server message type as `sql_event.created`.
- `ARCHITECTURE.md` says WebSocket filters are applied by the WebSocket layer, but Issue 036 owns WebSocket filters. This task should keep filtering out of scope.
- Current `CaptureEventReceiver` is single-consumer `mpsc`; multiple WebSocket clients need an API-side broadcast/fan-out boundary instead of each client reading from the capture receiver directly.
- Product decision: a WebSocket client must send a `subscribe` message before it receives SQL events. Connections do not auto-subscribe after upgrade.
- Product decision: malformed, unsupported, or wrong-version subscription messages are ignored while the socket continues waiting for a valid `subscribe` message.

## Requirements

- Add a protocol-neutral SQL event broadcast boundary for API/WebSocket subscribers.
- Expose that broadcast boundary through `ApiState` so tests and future runtime composition can publish live events into the WebSocket layer.
- Serialize live SQL event messages as JSON WebSocket text frames with:
  - `type: "sql_event.created"`,
  - `version: 1`,
  - `payload` containing an API-facing SQL event summary.
- Reuse existing SQL event response mapping conventions where practical instead of inventing a second DTO style.
- Support at least one WebSocket subscriber receiving one published SQL event.
- Require a client text message with `type: "subscribe"` and `version: 1` before delivering `sql_event.created` messages to that socket.
- Ignore invalid subscription text messages before subscription; do not close the socket or emit WebSocket error frames in this task.
- Preserve the existing initial ping heartbeat and clean close/disconnect lifecycle.
- Keep this task protocol-neutral; do not add MySQL-specific fields or behavior.
- Do not implement filters, replay, authentication, storage persistence, statistics streaming, or frontend client code.

## Acceptance Criteria

- [x] `/ws/sql` subscribers can receive a `sql_event.created` text message after a SQL event is published.
- [x] A WebSocket client does not receive SQL events before it sends a valid `subscribe` message.
- [x] A WebSocket client receives SQL events after it sends a valid `subscribe` message.
- [x] Invalid subscription messages are ignored and the socket can still subscribe later with a valid message.
- [x] Server event JSON includes `type`, `version`, and `payload`.
- [x] The payload uses existing SQL event API field naming and redaction assumptions.
- [x] At least one integration-style test covers one subscriber receiving one event.
- [x] Existing WebSocket foundation tests still pass.
- [x] Existing REST endpoint tests still pass.
- [x] `cargo fmt --check` passes.
- [x] `cargo test -p sql-lens-api` passes.
- [x] `cargo test --workspace` passes.
- [x] `cargo clippy --workspace --all-targets -- -D warnings` passes.

## Out of Scope

- WebSocket filters from Issue 036.
- Historical replay for newly connected subscribers.
- Multi-node broadcast.
- Authentication and authorization.
- WebSocket statistics stream.
- Browser/frontend WebSocket client.
- Connecting the proxy runtime capture receiver to API broadcast in `sql-lens-app`.
