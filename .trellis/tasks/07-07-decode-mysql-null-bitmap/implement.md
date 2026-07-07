# Decode MySQL NULL bitmap plan

## Checklist

- [x] Read backend specs before implementation.
- [x] Read NULL bitmap research notes.
- [x] Add MySQL execute parser module.
- [x] Add `MysqlNullBitmap`.
- [x] Add `MysqlExecuteParseError`.
- [x] Decode mixed NULL and non-NULL parameter positions.
- [x] Decode all non-NULL bitmap.
- [x] Handle zero parameter count.
- [x] Return structured error for truncated bitmap bytes.
- [x] Re-export parser types from `sql-lens-protocol-mysql`.
- [x] Extend `MysqlStatementExecuteEnvelope` with NULL parameter indexes.
- [x] Decode NULL bitmap for known statement IDs.
- [x] Keep unknown statement IDs non-fatal without bitmap decoding.
- [x] Keep malformed NULL bitmap non-fatal at adapter level.
- [x] Ensure NULL bitmap decoding emits zero events.
- [x] Keep existing query, prepare, and execute envelope tests passing.
- [x] Update backend spec with NULL bitmap contract.
- [x] Run `rtk cargo fmt --check`.
- [x] Run `rtk cargo test -p sql-lens-protocol-mysql`.
- [x] Run `rtk cargo test --workspace`.
- [x] Run `rtk cargo clippy --workspace --all-targets -- -D warnings`.

## Validation Commands

```bash
rtk cargo fmt --check
rtk cargo test -p sql-lens-protocol-mysql
rtk cargo test --workspace
rtk cargo clippy --workspace --all-targets -- -D warnings
```

## Risk Notes

- Do not store raw parameter payload bytes.
- Do not decode `new_params_bind_flag`.
- Do not decode parameter types or values.
- Do not render expanded SQL.
- Do not emit SQL events.
- Do not add new dependencies.
