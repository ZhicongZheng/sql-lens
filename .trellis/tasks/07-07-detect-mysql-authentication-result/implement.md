# Detect MySQL authentication result plan

## Checklist

- [x] Read backend specs before implementation.
- [x] Add authentication status/result safe metadata types.
- [x] Add structured auth result parse error type.
- [x] Implement OK packet detection.
- [x] Implement ERR packet metadata parsing.
- [x] Treat unsupported auth continuation packets as non-fatal and non-transitioning.
- [x] Re-export auth result parser/contracts from `lib.rs`.
- [x] Extend `MysqlConnectionPhase` with authenticated and auth-failed phases.
- [x] Extend `MysqlConnectionState` with sanitized auth result metadata.
- [x] Update `observe_backend_bytes` to detect auth result only after client handshake.
- [x] Keep malformed/incomplete observed backend bytes non-fatal and non-transitioning.
- [x] Add parser unit tests.
- [x] Add adapter state-transition tests.
- [x] Update backend spec with authentication result contract.
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

- Do not store raw auth result payloads.
- Do not log backend error packets.
- Do not implement auth switch or continuation flows.
- Do not parse SQL commands.
- Do not emit SQL events from authentication observation.

## Review Gate

Before implementation starts, confirm:

- Auth result observation requires `ClientHandshakeSeen`.
- OK/ERR detection is enough for this task.
- Unsupported auth continuation packets remain non-fatal.
