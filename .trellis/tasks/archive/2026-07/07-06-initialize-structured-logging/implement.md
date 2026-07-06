# Structured Logging Implementation Plan

## Preconditions

- Planning artifacts are reviewed.
- Task is activated with `task.py start`.
- Implementation remains limited to structured logging initialization in `sql-lens-app`.

## Files To Modify

- `crates/sql-lens-app/Cargo.toml`
- `crates/sql-lens-app/src/main.rs`
- `crates/sql-lens-app/tests/cli.rs`
- `Cargo.lock`
- `.trellis/spec/backend/logging-guidelines.md`
- `.trellis/spec/backend/index.md`

## Checklist

1. [x] Add `tracing` dependency to `sql-lens-app`.
2. [x] Add `tracing-subscriber` dependency with JSON formatter support.
3. [x] Import `LoggingConfig`, `LoggingFormat`, and `LoggingLevel` from `sql-lens-config`.
4. [x] Add `init_logging(&LoggingConfig) -> Result<(), AppError>`.
5. [x] Map `LoggingLevel` to `tracing_subscriber::filter::LevelFilter`.
6. [x] Select JSON or pretty formatter from `LoggingFormat`.
7. [x] Use `try_init` and convert initialization errors into `AppError::LoggingInit`.
8. [x] Call logging initialization after config validation succeeds.
9. [x] Emit one info-level startup-check log event after initialization.
10. [x] Update CLI integration tests for success stderr behavior.
11. [x] Add JSON formatter smoke test.
12. [x] Add pretty formatter smoke test.
13. [x] Add log-level filtering smoke test.
14. [x] Update backend logging spec with the new concrete contract.
15. [x] Run validation.
16. [x] Verify no runtime startup or out-of-scope logging behavior was introduced.

## Validation Results

- [x] `rtk cargo fmt --check`
- [x] `rtk cargo check --workspace`
- [x] `rtk cargo test --workspace`
- [x] `rtk cargo clippy --workspace --all-targets -- -D warnings`
- [x] `rtk cargo tree -p sql-lens-app`
- [x] `rtk rg -n "tokio|axum|notify|rusqlite|hyper|opentelemetry|EnvFilter|RUST_LOG" crates/sql-lens-app`
- [x] `rtk python3 ./.trellis/scripts/task.py validate .trellis/tasks/07-06-initialize-structured-logging`

## Validation Commands

```bash
rtk cargo fmt --check
rtk cargo check --workspace
rtk cargo test --workspace
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk cargo tree -p sql-lens-app
rtk rg -n "tokio|axum|notify|rusqlite|hyper|opentelemetry|EnvFilter|RUST_LOG" crates/sql-lens-app
rtk python3 ./.trellis/scripts/task.py validate .trellis/tasks/07-06-initialize-structured-logging
```

## Review Gate

Do not implement:

- Proxy startup.
- API startup.
- Web UI startup.
- Runtime service composition.
- Environment variable overrides.
- `RUST_LOG` / env filter behavior.
- Hot reload.
- File logging.
- OpenTelemetry.
- SQL redaction logic.

## Commit Plan

When implementation and validation pass, create one work commit:

```text
feat: initialize structured logging
```

Do not archive the task until the work commit exists.
