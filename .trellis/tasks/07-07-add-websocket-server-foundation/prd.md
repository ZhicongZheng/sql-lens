# Add WebSocket server foundation

## Goal

Implement Issue 034 design: add the foundation for WebSocket connections at `GET /ws/sql` so future live SQL event streaming can be built on a stable upgrade, heartbeat, and disconnect lifecycle.

## Background

- `API.md` documents `GET /ws/sql` as the SQL Events Stream endpoint.
- Issue 034 acceptance criteria require `/ws/sql` to accept connections, handle disconnects cleanly, and provide basic ping/pong or heartbeat behavior.
- Current `sql-lens-api` has REST route modules only; no WebSocket route module exists yet.
- Current `sql-lens-api` depends on `axum = "0.8"` without the `ws` feature.
- Axum 0.8 provides `WebSocketUpgrade`, `WebSocket`, and `Message::Ping` for WebSocket upgrade and heartbeat handling.

## Requirements

- Add WebSocket upgrade support for `GET /ws/sql`.
- Accept WebSocket connections without requiring SQL capture/storage fan-out yet.
- Send an initial heartbeat ping after upgrade.
- Continue reading client messages until the client disconnects, sends close, or the socket errors.
- Treat disconnects and close frames as normal lifecycle completion, not API errors.
- Keep request ID middleware behavior for the upgrade HTTP response.
- Keep the first foundation protocol-neutral; do not add MySQL-only subscription behavior.
- Keep the module focused on connection lifecycle; do not implement SQL event publishing, filtering, replay, authentication, or statistics streaming.

## Acceptance Criteria

- [ ] `GET /ws/sql` is registered in the API router.
- [ ] A valid WebSocket upgrade request receives a switching-protocols response.
- [ ] The upgraded task sends at least one server ping heartbeat.
- [ ] Client close/disconnect completes cleanly without panics.
- [ ] Non-WebSocket requests to `/ws/sql` return an appropriate HTTP error from Axum.
- [ ] Existing REST endpoint tests still pass.
- [ ] `cargo fmt --check` passes.
- [ ] `cargo test --workspace` passes.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes.

## Out of Scope

- SQL event broadcast channel.
- Subscription message parsing.
- WebSocket filters.
- WebSocket statistics stream at `/ws/statistics`.
- Authentication and authorization.
- Backpressure policy for fan-out.
- Frontend WebSocket client.
