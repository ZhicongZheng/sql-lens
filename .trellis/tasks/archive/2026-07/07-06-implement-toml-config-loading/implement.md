# TOML Config Loading Implementation Plan

## Preconditions

- Planning artifacts are reviewed.
- Task is activated with `task.py start`.
- Implementation remains limited to `sql-lens-config` TOML loading and tests.

## Files To Modify

- `crates/sql-lens-config/Cargo.toml`
- `crates/sql-lens-config/src/lib.rs`
- `Cargo.lock`

## Checklist

1. Add `toml` dependency to `sql-lens-config` with minimal parse/serde features.
2. Add serde default and unknown-field behavior to config model structs.
3. Add `ConfigLoadError`.
4. Implement `Display` and `std::error::Error` for `ConfigLoadError`.
5. Implement `SqlLensConfig::from_path`.
6. Implement `SqlLensConfig::from_toml_str`.
7. Add test helpers for standard-library temporary config files.
8. Add tests for valid TOML file loading.
9. Add tests for invalid TOML parse error.
10. Add tests for missing-file read error.
11. Add tests for partial TOML default fallback.
12. Add tests for unknown-field rejection.
13. Run validation.
14. Verify no out-of-scope config behavior was introduced.

## Validation Commands

```bash
rtk cargo fmt --check
rtk cargo check --workspace
rtk cargo test --workspace
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk cargo tree -p sql-lens-config
rtk rg -n "notify|clap|tokio|axum|rusqlite|sql-lens-core" crates/sql-lens-config Cargo.toml
rtk python3 ./.trellis/scripts/task.py validate .trellis/tasks/07-06-implement-toml-config-loading
```

## Review Gate

Do not implement:

- Environment overrides.
- Config semantic validation.
- CLI `--config`.
- Runtime startup.
- Hot reload.
- File watching.
- SQLite runtime settings.
- Logging initialization.
- Proxy or API startup.

## Commit Plan

When implementation and validation pass, create one work commit:

```text
feat: add toml config loading
```

Do not archive the task until the work commit exists.
