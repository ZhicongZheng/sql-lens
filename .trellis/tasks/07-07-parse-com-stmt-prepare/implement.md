# Parse COM_STMT_PREPARE plan

## Checklist

- [x] Read backend specs before implementation.
- [x] Read COM_STMT_PREPARE research notes.
- [x] Add `MYSQL_COM_STMT_PREPARE`.
- [x] Add `MysqlComStmtPrepare`.
- [x] Extend command parser to return a command enum or equivalent narrow dispatch.
- [x] Keep existing `COM_QUERY` parser behavior and tests.
- [x] Parse valid `COM_STMT_PREPARE` SQL template.
- [x] Cover empty template and invalid UTF-8 template tests.
- [x] Add MySQL pending statement-prepare state.
- [x] Store pending statement prepare after authentication.
- [x] Keep prepare before authentication ignored.
- [x] Keep unsupported and malformed commands non-fatal.
- [x] Ensure prepare emits zero events.
- [x] Update backend spec with COM_STMT_PREPARE parser contract.
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

- Do not parse backend prepare OK response.
- Do not introduce statement ID mappings in this task.
- Do not emit prepared-statement events yet.
- Do not add new dependencies.
- Keep MySQL-specific prepared-statement state inside the MySQL crate.
