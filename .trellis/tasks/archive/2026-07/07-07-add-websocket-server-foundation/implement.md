# Add WebSocket server foundation plan

## Checklist

- [x] Read backend specs before implementation.
- [x] Enable Axum `ws` feature for `sql-lens-api`.
- [x] Add `websocket.rs` route module with `SQL_WS_PATH`.
- [x] Implement `GET /ws/sql` upgrade handler.
- [x] Implement minimal socket lifecycle:
  - [x] Send initial ping.
  - [x] Read until close, disconnect, or socket error.
  - [x] Ignore text/binary messages for now.
- [x] Merge WebSocket routes in `server::router_with_state` before fallback.
- [x] Export `SQL_WS_PATH` from `sql-lens-api`.
- [x] Add focused tests for WebSocket connection and clean close.
- [x] Add rejection test for plain HTTP request to `/ws/sql`.
- [x] Update backend spec with WebSocket foundation contract.
- [x] Run `rtk cargo fmt --check`.
- [x] Run `rtk cargo test -p sql-lens-api`.
- [x] Run `rtk cargo test --workspace`.
- [x] Run `rtk cargo clippy --workspace --all-targets -- -D warnings`.

## Validation Commands

```bash
rtk cargo fmt --check
rtk cargo test -p sql-lens-api
rtk cargo test --workspace
rtk cargo clippy --workspace --all-targets -- -D warnings
```

## Risk Notes

- Keep this task to upgrade/lifecycle only; do not parse subscription payloads.
- Avoid periodic heartbeat policy until the product needs timeout semantics.
- Do not add SQL event broadcast channels in this task.
- WebSocket upgrade extractor errors may not use the REST JSON error envelope; that is acceptable for protocol handshake failures.

## Review Gate

Before implementation starts, confirm:

- `GET /ws/sql` foundation only.
- Initial ping heartbeat only, no periodic heartbeat loop.
- No subscription JSON parsing until Issue 035.
