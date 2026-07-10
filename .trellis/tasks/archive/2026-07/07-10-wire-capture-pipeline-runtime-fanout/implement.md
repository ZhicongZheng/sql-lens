# Implementation Plan

1. Add `CaptureConfig` and a config-owned overload-policy enum to
   `sql-lens-config`; re-export them, validate nonzero capacity, document the
   `[capture]` TOML section, and add config tests.
2. Extend capture pipeline counters with a closed-receiver count and update
   unit tests without changing the producer's non-blocking API.
3. Add app runtime capture composition: construct the pipeline, spawn the
   receiver consumer with an explicit shutdown signal, carry its task handle in
   `MinimalMysqlRuntime`, and drain already-accepted events before SQLite worker
   teardown without waiting for detached session publishers.
4. Pass publisher clones through proxy target and connection forwarding paths;
   replace direct `store_sql_events` awaits in packet forwarding with a
   non-blocking publication helper.
5. Move the existing classify/store/broadcast/persist sequence into the
   receiver consumer and add focused app tests for fan-out, overload outcomes,
   shared runtime construction, and drain behavior.
6. Update `CONFIG.md`, the backend capture quality spec, and the task PRD.

## Validation

```text
rtk cargo fmt --check
rtk cargo test -p sql-lens-config
rtk cargo test -p sql-lens-capture
rtk cargo test -p sql-lens-app --lib
rtk cargo test --workspace
rtk cargo clippy --workspace --all-targets -- -D warnings
```

## Rollback

The runtime integration is isolated to `sql-lens-app` construction and event
publication. Reverting the app composition and `[capture]` config addition
returns event fan-out to the current direct path without requiring data or API
migration.
