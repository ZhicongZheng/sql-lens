# Implement — Issue 082: Slow SQL Classification

## Steps

1. Read backend specs for core event contracts, config contracts, capture
   pipeline contracts, live statistics, and app runtime fan-out.
2. Add `slow_threshold_ms` to `ProxyConfig` with a default of `500`.
3. Add config tests for default and TOML parsing of the threshold.
4. Add a `SlowQueryClassifier` or equivalent function in `sql-lens-capture`.
5. Unit test below, equal, above, error, unknown, and already-slow behavior.
6. Wire app `store_sql_events` to classify before broadcast/storage and record
   classified events into `LiveStatistics`.
7. Add an app/API test proving a classified event is stored as `slow` and
   statistics slow count increments.
8. Update backend specs if the classifier contract is durable.
9. Validate:
   - `rtk cargo fmt --check`
   - `rtk cargo test -p sql-lens-config`
   - `rtk cargo test -p sql-lens-capture`
   - `rtk cargo test -p sql-lens-app`
   - `rtk cargo test --workspace`
   - `rtk cargo clippy --workspace --all-targets -- -D warnings`

## Candidate Files

- `crates/sql-lens-config/src/model.rs`
- `crates/sql-lens-config/src/tests.rs`
- `crates/sql-lens-capture/src/lib.rs`
- `crates/sql-lens-capture/src/pipeline.rs` or a new focused module
- `crates/sql-lens-app/src/lib.rs`
- `.trellis/spec/backend/quality-guidelines.md`

## Rollback

Remove the classifier module, config field, app fan-out wiring, and related
tests. Core `CaptureStatus::Slow` and API status handling already exist and do
not need rollback.

## Validation Results

- `rtk cargo fmt --check` passed.
- `rtk cargo test -p sql-lens-config` passed.
- `rtk cargo test -p sql-lens-capture` passed.
- `rtk cargo test -p sql-lens-app` passed.
- `rtk cargo test --workspace` passed.
- `rtk cargo clippy --workspace --all-targets -- -D warnings` passed.
