# CLI Entry Point Implementation Plan

## Preconditions

- Planning artifacts are reviewed.
- Task is activated with `task.py start`.
- Implementation remains limited to the initial CLI entry point.

## Files To Modify

- `crates/sql-lens-app/Cargo.toml`
- `crates/sql-lens-app/src/main.rs`
- `Cargo.lock`

## Files To Add

- `crates/sql-lens-app/tests/cli.rs`

## Checklist

1. [x] Add `clap` derive dependency to `sql-lens-app`.
2. [x] Add `sql-lens-config` path dependency to `sql-lens-app`.
3. [x] Implement `Cli` with `--config <FILE>` defaulting to `sql-lens.toml`.
4. [x] Implement synchronous `run(cli)` that loads and validates config.
5. [x] Implement `AppError` for load and validation failures.
6. [x] Make `main` parse args, run startup checks, print errors to stderr, and return `ExitCode`.
7. [x] Add standard-library integration test helpers for temp config files.
8. [x] Add integration test for `--version`.
9. [x] Add integration test for valid config path success.
10. [x] Add integration test for missing config path failure.
11. [x] Add integration test for validation failure.
12. [x] Run validation.
13. [x] Verify no runtime startup or other out-of-scope behavior was introduced.

## Validation Results

- [x] `rtk cargo fmt --check`
- [x] `rtk cargo check --workspace`
- [x] `rtk cargo test --workspace`
- [x] `rtk cargo clippy --workspace --all-targets -- -D warnings`
- [x] `rtk cargo tree -p sql-lens-app`
- [x] `rtk rg -n "tokio|axum|notify|rusqlite|hyper|tracing_subscriber" crates/sql-lens-app`
- [x] `rtk python3 ./.trellis/scripts/task.py validate .trellis/tasks/07-06-add-cli-entry-point`

## Validation Commands

```bash
rtk cargo fmt --check
rtk cargo check --workspace
rtk cargo test --workspace
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk cargo tree -p sql-lens-app
rtk rg -n "tokio|axum|notify|rusqlite|hyper|tracing_subscriber" crates/sql-lens-app
rtk python3 ./.trellis/scripts/task.py validate .trellis/tasks/07-06-add-cli-entry-point
```

## Review Gate

Do not implement:

- Proxy startup.
- API startup.
- Web UI startup.
- Runtime service composition.
- Logging initialization.
- Environment variable overrides.
- Hot reload.
- Shell completions.
- Subcommands.

## Commit Plan

When implementation and validation pass, create one work commit:

```text
feat: add cli entry point
```

Do not archive the task until the work commit exists.
