# Parse COM_STMT_EXECUTE envelope plan

## Checklist

- [x] Resolve unknown statement ID behavior with the user.
- [x] Read backend specs before implementation.
- [x] Read execute envelope research notes.
- [x] Add `MYSQL_COM_STMT_EXECUTE`.
- [x] Add `MysqlComStmtExecute`.
- [x] Extend client command parser dispatch.
- [x] Parse valid execute statement ID.
- [x] Parse execute flags.
- [x] Parse iteration count.
- [x] Detect trailing parameter payload bytes.
- [x] Add structured parse errors for incomplete execute envelopes.
- [x] Add MySQL last execute envelope state.
- [x] Store execute envelope after authentication.
- [x] Link known statement ID to prepared statement metadata.
- [x] Handle unknown statement ID non-fatally.
- [x] Keep execute before authentication ignored.
- [x] Ensure execute emits zero events.
- [x] Keep existing query and prepare tests passing.
- [x] Update backend spec with execute envelope contract.
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

- Do not decode NULL bitmap in this task.
- Do not decode parameter types or values.
- Do not render expanded SQL.
- Do not emit SQL events.
- Do not add new dependencies.
