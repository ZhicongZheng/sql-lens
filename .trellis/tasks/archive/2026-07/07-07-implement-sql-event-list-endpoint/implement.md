# Implement SQL Event List Endpoint Plan

## Checklist

- [x] Start the task with `rtk python3 ./.trellis/scripts/task.py start .trellis/tasks/07-07-implement-sql-event-list-endpoint`.
- [x] Load `trellis-before-dev` before editing code.
- [x] Extend `sql-lens-storage::SqlEventFilter` with `client_addr` and `fingerprint`.
- [x] Add storage tests for `client_addr` and `fingerprint` filtering.
- [x] Add `sql-lens-storage` and `sql-lens-core` dependencies to `sql-lens-api`.
- [x] Enable needed Tokio `sync` feature for API state.
- [x] Add API state module with `ApiState`.
- [x] Add SQL event API module:
  - path constant
  - query params
  - response DTOs
  - cursor encode/decode
  - status/kind mapping
  - metadata value mapping
  - handler
  - route builder
- [x] Update `server::router()` to call `router_with_state(ApiState::default())`.
- [x] Add `router_with_state(ApiState)` and register health + SQL event routes under request ID middleware.
- [x] Add endpoint tests:
  - empty list works
  - populated list response schema matches API.md fields
  - filter query maps to storage
  - pagination cursor returns older page
  - invalid cursor returns HTTP 400
  - invalid duration range returns HTTP 400
- [x] Run narrow crate tests.
- [x] Run full workspace validation.
- [x] Update backend spec with the SQL event list endpoint contract if implementation confirms the design.

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

- Response DTOs must not leak Rust enum variant names.
- Avoid holding a storage lock across `.await` points after the query is complete.
- Keep query parsing local and simple; do not create a broad API framework before more endpoints exist.
- Do not change `sql-lens-app` runtime behavior.
- Cursor format becomes a public API contract; keep it documented and tested.

## Rollback Points

- If API state design becomes too large, keep storage filter extension and pause before endpoint registration.
- If error response plumbing becomes noisy, keep response header request IDs and use minimal documented JSON error bodies.
