# Add WebSocket filters plan

## Checklist

- [x] Read backend specs before implementation.
- [x] Update WebSocket subscribe request DTO to include optional filters.
- [x] Add WebSocket subscription filter DTO with strict filter-field validation.
- [x] Parse supported filters into core domain types.
- [x] Add `subscription.error` server message DTO.
- [x] Return `subscription.error` for invalid filters.
- [x] Keep socket open and waiting after invalid filter errors.
- [x] Apply filters before sending `sql_event.created`.
- [x] Keep unfiltered subscription behavior unchanged.
- [x] Add unit tests for filter parsing and matching.
- [x] Add WebSocket tests for matching and non-matching protocol/status/database/duration events.
- [x] Add WebSocket test for invalid filters returning `subscription.error`.
- [x] Add WebSocket test proving valid subscribe still works after a filter error.
- [x] Update API documentation to remove the "filters not yet applied" note.
- [x] Update backend spec with WebSocket filter contract.
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

- Do not modify storage filter contracts unless implementation proves local WebSocket matching is insufficient.
- Do not add historical replay.
- Do not close sockets on invalid filters; send error and keep waiting.
- Do not add MySQL-specific filters.
- Keep unsupported filter fields out of this task even if REST supports more fields.

## Review Gate

Before implementation starts, confirm:

- Subscription errors use `subscription.error`.
- Supported filters are only protocol, status, database, min duration, and max duration.
- Invalid filters send an error but keep the socket open.
