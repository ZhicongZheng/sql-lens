# Store prepared statement state per connection plan

## Checklist

- [x] Resolve close cleanup scope with the user.
- [x] Read backend specs before implementation.
- [x] Add `MysqlPreparedStatement`.
- [x] Add a per-connection prepared statement map to `MysqlConnectionState`.
- [x] Initialize the map as empty.
- [x] Add narrow read accessors.
- [x] Insert mapping after successful prepare OK.
- [x] Do not insert mapping after prepare ERR.
- [x] Replace mapping when the same statement ID is prepared again.
- [x] Add tests for empty initial map.
- [x] Add tests for successful prepare insertion.
- [x] Add tests for failed prepare no insertion.
- [x] Add tests for same-ID replacement.
- [x] Add tests proving cross-connection isolation.
- [x] Keep existing `COM_QUERY` and prepare response behavior passing.
- [x] Update backend spec if implementation confirms this contract.
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

- Do not add a shared protocol close hook unless the user explicitly chooses that broader design.
- Do not parse execute packets in this task.
- Do not expose prepared statement mappings outside the MySQL crate yet.
- Do not add new dependencies.
