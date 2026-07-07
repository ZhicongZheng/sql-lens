# Implement SQL Event Detail Endpoint Plan

## Checklist

- [x] Start the task with `rtk python3 ./.trellis/scripts/task.py start .trellis/tasks/07-07-implement-sql-event-detail-endpoint`.
- [x] Load `trellis-before-dev` before editing code.
- [x] Extend `sql_events.rs` routes with `GET /api/v1/sql-events/{id}`.
- [x] Add `SqlEventDetailResponse` and supporting DTOs.
- [x] Reuse existing summary/status/kind/rows/metadata mapping helpers.
- [x] Add `ApiEndpointError::not_found`.
- [x] Add tests:
  - existing event returns HTTP 200.
  - missing event returns HTTP 404 and `NOT_FOUND`.
  - response contains parameters, timings, result, error, and metadata.
  - request ID header is present.
- [x] Re-export public detail DTOs from `lib.rs`.
- [x] Update backend spec with detail endpoint contract.
- [x] Run validation commands.

## Validation Commands

```bash
rtk cargo test -p sql-lens-api
rtk cargo fmt --check
rtk cargo check --workspace
rtk cargo test --workspace
rtk cargo clippy --workspace --all-targets -- -D warnings
```

## Risk Points

- Avoid duplicating list DTO mapping logic into a divergent second implementation.
- Do not expose Rust enum variant names in JSON.
- Keep missing-event behavior as 404, not empty 200.
- Do not change storage lookup behavior.
