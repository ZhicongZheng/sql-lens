# Core Domain Models Implementation Plan

## Preconditions

- User approves planning artifacts.
- `task.py start` is run before implementation.
- Implementation remains limited to `sql-lens-core`.

## Files To Modify

- `crates/sql-lens-core/Cargo.toml`
- `crates/sql-lens-core/src/lib.rs`

## Files To Add

- `crates/sql-lens-core/src/ids.rs`
- `crates/sql-lens-core/src/time.rs`
- `crates/sql-lens-core/src/metadata.rs`
- `crates/sql-lens-core/src/event.rs`
- `crates/sql-lens-core/src/error.rs`

## Checklist

1. Add `serde` with derive feature to `sql-lens-core`.
2. Add ID newtypes.
3. Add timestamp and duration newtypes.
4. Add protocol/database metadata types.
5. Add SQL event, connection, prepared statement, parameter, timing, result, status, and kind types.
6. Add API error and SQL error summary types.
7. Re-export public types from `lib.rs`.
8. Add lightweight unit tests.
9. Run validation.
10. Verify no out-of-scope logic was introduced.

## Validation Commands

```bash
rtk cargo fmt --check
rtk cargo check --workspace
rtk cargo test --workspace
rtk rg -n "serde_json|uuid|time =" crates/sql-lens-core Cargo.toml
```

## Review Gate

Do not implement:

- Protocol adapter traits.
- MySQL parser details.
- Storage query filters.
- Statistics aggregation.
- Replay models.
- REST or WebSocket handlers.
- OpenAPI generation.
- Redaction rule engine.

