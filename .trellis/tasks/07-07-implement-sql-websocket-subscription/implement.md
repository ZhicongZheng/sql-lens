# Implement SQL WebSocket subscription plan

## Checklist

- [x] Read backend specs before implementation.
- [x] Add API-owned SQL event broadcast types.
- [x] Add broadcaster field and accessor to `ApiState`.
- [x] Keep existing `ApiState` constructors compatible by installing a default broadcaster.
- [x] Move `serde_json` to normal `sql-lens-api` dependencies.
- [x] Add WebSocket subscription request DTO.
- [x] Add `sql_event.created` server message DTO wrapping `SqlEventSummaryResponse`.
- [x] Require valid `subscribe` before forwarding events.
- [x] Preserve initial heartbeat ping.
- [x] Forward broadcast SQL events as JSON text frames after subscription.
- [x] Handle socket close/disconnect/error cleanly.
- [x] Add broadcaster unit tests.
- [x] Add WebSocket test proving no event is delivered before subscribe.
- [x] Add WebSocket test proving one subscriber receives one event after subscribe.
- [x] Update backend spec with SQL WebSocket subscription contract.
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

- Do not wire proxy capture runtime to API broadcast in this task.
- Do not implement filters; Issue 036 owns filter parsing and matching.
- Do not block broadcast publishers on WebSocket clients.
- Do not add MySQL-specific payload fields.
- Keep WebSocket error behavior simple until an error frame contract exists.

## Review Gate

Before implementation starts, confirm:

- Clients must send `subscribe` before receiving SQL events.
- Invalid subscription messages are ignored while waiting for a valid subscribe.
- SQL event payload reuses `SqlEventSummaryResponse`.
