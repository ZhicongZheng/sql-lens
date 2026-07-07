# Parse COM_QUERY plan

## Checklist

- [x] Read backend specs before implementation.
- [x] Add a MySQL command parser module.
- [x] Add command kind and client command metadata types.
- [x] Implement `COM_QUERY` command-byte detection.
- [x] Extract SQL text as UTF-8.
- [x] Return structured errors for empty payload and invalid UTF-8.
- [x] Treat unsupported command bytes as non-fatal.
- [x] Re-export command parser/contracts from `lib.rs`.
- [x] Extend `MysqlConnectionState` with last parsed client command metadata.
- [x] Update `observe_client_bytes` to parse commands only after `Authenticated`.
- [x] Keep malformed command packets non-fatal and non-transitioning.
- [x] Add parser unit tests.
- [x] Add adapter state tests.
- [x] Update backend spec with `COM_QUERY` parser contract.
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

- Do not emit SQL events in this task.
- Do not measure duration or inspect backend responses.
- Do not store raw command payload bytes.
- Do not decode using connection character set yet; UTF-8 only for this first layer.
- Do not parse prepared statements or other command kinds.

## Review Gate

Before implementation starts, confirm:

- Command parsing is gated on `Authenticated`.
- Unsupported commands remain non-fatal.
- `COM_QUERY` parsing stores MySQL-specific state only; protocol-neutral event capture comes later.
