# Parse MySQL error packet summary plan

## Checklist

- [x] Read backend specs before implementation.
- [x] Read ERR packet research notes.
- [x] Add a small MySQL ERR packet parser module.
- [x] Add `MysqlErrPacketSummary`.
- [x] Decode MySQL error code.
- [x] Decode SQLSTATE when present.
- [x] Decode and sanitize error message.
- [x] Re-export parser contracts needed by crate tests or future parser users.
- [x] Integrate ERR summary parsing into failed `COM_QUERY` finalization.
- [x] Populate `SqlEvent.error` for failed ERR events.
- [x] Add MySQL error code metadata to `ErrorSummary.metadata`.
- [x] Keep malformed ERR summary parsing non-fatal at adapter level.
- [x] Keep OK event behavior unchanged.
- [x] Add parser unit tests.
- [x] Add adapter event tests for error summary and metadata.
- [x] Update backend spec with ERR packet summary contract.
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

- Do not add new dependencies.
- Do not log raw database error messages.
- Do not add general-purpose redaction rules in this task.
- Do not add MySQL-only fields to protocol-neutral event structs.
- Do not make malformed ERR summary parsing fatal to proxy observation.
