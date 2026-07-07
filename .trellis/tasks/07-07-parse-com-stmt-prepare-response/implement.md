# Parse COM_STMT_PREPARE response plan

## Checklist

- [x] Confirm task boundary with the user.
- [x] Read backend specs before implementation.
- [x] Read prepare response research notes.
- [x] Add a MySQL prepare response parser module.
- [x] Add `MysqlComStmtPrepareOk`.
- [x] Add `MysqlComStmtPrepareResponse`.
- [x] Add structured prepare response parse errors.
- [x] Parse valid `COM_STMT_PREPARE_OK` statement ID, column count, and parameter count.
- [x] Cover incomplete prepare OK payload tests.
- [x] Reuse existing ERR packet parser for prepare ERR responses.
- [x] Add MySQL last statement-prepare outcome state.
- [x] Consume pending prepare after successful prepare OK.
- [x] Consume pending prepare after prepare ERR.
- [x] Keep malformed prepare responses non-fatal and leave pending prepare intact.
- [x] Keep backend responses without pending prepare non-fatal.
- [x] Ensure prepare response parsing emits zero events.
- [x] Keep existing `COM_QUERY` response tests passing.
- [x] Update backend spec with prepare response parser contract if implementation confirms this design.
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

- Do not create the per-connection prepared statement map in this task.
- Do not parse parameter or column definition packets.
- Do not emit prepared-statement events yet.
- Do not add new dependencies.
- Do not change protocol-neutral core models.
