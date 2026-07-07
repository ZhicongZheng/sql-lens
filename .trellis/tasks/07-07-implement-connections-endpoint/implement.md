# Implement Connections Endpoint Plan

## Checklist

- [x] Start the task with `rtk python3 ./.trellis/scripts/task.py start .trellis/tasks/07-07-implement-connections-endpoint`.
- [x] Load `trellis-before-dev` before editing code.
- [x] Add `connection_store.rs` in `sql-lens-storage`.
- [x] Re-export `ConnectionStore` and `ConnectionUpsertOutcome` from `sql-lens-storage`.
- [x] Add storage tests:
  - upsert active connection
  - update existing connection to closed
  - list recent newest-first
  - get existing
  - missing/evicted returns none
- [x] Extend `ApiState` with `connection_store`.
- [x] Add `connections.rs` in `sql-lens-api`.
- [x] Add list/detail routes and DTOs.
- [x] Re-export public connection DTOs and path constants.
- [x] Register connection routes in `router_with_state`.
- [x] Add API tests:
  - list returns active and closed connections
  - detail returns existing connection
  - missing detail returns 404 `NOT_FOUND`
  - invalid limit returns 400
  - request ID header is present
- [x] Update backend spec with connection store and endpoint contracts.
- [x] Run validation commands.

## Validation Commands

```bash
rtk cargo test -p sql-lens-storage
rtk cargo test -p sql-lens-api
rtk cargo fmt --check
rtk cargo check --workspace
rtk cargo test --workspace
rtk cargo clippy --workspace --all-targets -- -D warnings
```

## Risk Points

- Do not wire proxy runtime updates in this task.
- Do not add storage async dependencies.
- Keep list endpoint simple: no cursor/filter unless a later task needs it.
- Preserve `ApiState::new(event_store)` compatibility.
- Do not expose Rust enum variant names in JSON.
