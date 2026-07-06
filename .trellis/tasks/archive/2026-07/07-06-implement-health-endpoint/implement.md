# Implement Health Endpoint Plan

## Checklist

- [x] Start the task with `rtk python3 ./.trellis/scripts/task.py start .trellis/tasks/07-06-implement-health-endpoint`.
- [x] Load `trellis-before-dev` before editing code.
- [x] Add `serde` dependency to `sql-lens-api`.
- [x] Add `serde_json` as a dev-dependency for schema tests if needed.
- [x] Add `health.rs` with:
  - `HEALTH_PATH`
  - `HealthResponse`
  - `HealthState`
  - handler and route builder
- [x] Update `server::router()` to merge/register health route before request ID middleware.
- [x] Re-export public health contract types from `lib.rs`.
- [x] Add tests for health response schema and request ID header.
- [x] Run narrow API crate tests.
- [x] Run full workspace validation.

## Validation Commands

```bash
rtk cargo test -p sql-lens-api
rtk cargo fmt --check
rtk cargo check --workspace
rtk cargo test --workspace
rtk cargo clippy --workspace --all-targets -- -D warnings
```

## Risk Points

- Do not introduce real readiness semantics before storage/proxy runtime composition exists.
- Do not change `sql-lens-app` behavior in this task.
- Keep the response schema aligned with `API.md`.
