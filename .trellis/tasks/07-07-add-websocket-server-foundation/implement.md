# Add WebSocket server foundation plan

## Checklist

- [ ] Read backend specs before implementation.
- [ ] Enable Axum `ws` feature for `sql-lens-api`.
- [ ] Add `websocket.rs` route module with `SQL_WS_PATH`.
- [ ] Implement `GET /ws/sql` upgrade handler.
- [ ] Implement minimal socket lifecycle:
  - [ ] Send initial ping.
  - [ ] Read until close, disconnect, or socket error.
  - [ ] Ignore text/binary messages for now.
- [ ] Merge WebSocket routes in `server::router_with_state` before fallback.
- [ ] Export `SQL_WS_PATH` from `sql-lens-api`.
- [ ] Add focused tests for WebSocket connection and clean close.
- [ ] Add rejection test for plain HTTP request to `/ws/sql`.
- [ ] Update backend spec with WebSocket foundation contract.
- [ ] Run `rtk cargo fmt --check`.
- [ ] Run `rtk cargo test -p sql-lens-api`.
- [ ] Run `rtk cargo test --workspace`.
- [ ] Run `rtk cargo clippy --workspace --all-targets -- -D warnings`.

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
