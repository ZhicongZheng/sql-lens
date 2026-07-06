# Configuration Model Implementation Plan

## Preconditions

- User approves planning artifacts.
- `task.py start` is run before implementation.
- Implementation remains limited to config model structs, enums, defaults, and tests.

## Files To Modify

- `Cargo.toml`
- `Cargo.lock`

## Files To Add

- `crates/sql-lens-config/Cargo.toml`
- `crates/sql-lens-config/src/lib.rs`

## Checklist

1. Add `crates/sql-lens-config` to workspace members.
2. Create `sql-lens-config` crate manifest.
3. Add `serde` with derive feature.
4. Implement `SqlLensConfig` top-level struct.
5. Implement all section structs from `CONFIG.md`.
6. Implement config-owned enums.
7. Implement explicit defaults.
8. Add lightweight unit tests.
9. Run validation.
10. Verify out-of-scope logic was not introduced.

## Validation Commands

```bash
rtk cargo fmt --check
rtk cargo check --workspace
rtk cargo test --workspace
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk rg -n "toml|sql-lens-core|notify|sqlite|rusqlite" crates/sql-lens-config Cargo.toml
```

## Review Gate

Do not implement:

- TOML loading.
- Environment overrides.
- Config validation.
- CLI `--config`.
- Runtime startup.
- Hot reload.
- SQLite runtime settings.
- Proxy startup.
- API startup.

