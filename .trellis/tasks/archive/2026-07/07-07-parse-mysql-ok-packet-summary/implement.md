# Parse MySQL OK packet summary plan

## Checklist

- [x] Read backend specs before implementation.
- [x] Read OK packet research notes.
- [x] Add a small MySQL OK packet parser module.
- [x] Add length-encoded integer decoding scoped to OK packet parsing.
- [x] Add `MysqlOkPacketSummary`.
- [x] Re-export only the parser contracts needed by crate tests or future parser users.
- [x] Parse affected rows from fixture OK packets.
- [x] Parse status flags when present.
- [x] Integrate OK summary parsing into successful `COM_QUERY` finalization.
- [x] Populate `SqlEvent.result` for successful OK events.
- [x] Add `ok_status_flags` metadata when available.
- [x] Keep malformed OK summary parsing non-fatal at adapter level.
- [x] Keep ERR event result behavior unchanged.
- [x] Add parser unit tests.
- [x] Add adapter event tests for result summary and metadata.
- [x] Update backend spec with OK packet summary contract.
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
- Do not parse full result-set lifecycle.
- Do not add MySQL-only fields to protocol-neutral core models.
- Do not make malformed OK summary parsing fatal to proxy observation.
- Do not log packet payloads or raw SQL.
