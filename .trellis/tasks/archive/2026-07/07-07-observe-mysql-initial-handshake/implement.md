# Observe MySQL initial handshake plan

## Checklist

- [x] Read backend specs before implementation.
- [x] Add `handshake.rs` under `crates/sql-lens-protocol-mysql/src`.
- [x] Add `MysqlInitialHandshake` safe metadata type.
- [x] Add structured `MysqlHandshakeParseError` with `Display` and `Error`.
- [x] Implement `parse_initial_handshake`.
- [x] Re-export handshake parser contracts from `lib.rs`.
- [x] Add `MysqlConnectionPhase`.
- [x] Extend `MysqlConnectionState` with phase and sanitized handshake metadata.
- [x] Add read-only accessors for phase and initial handshake.
- [x] Update `observe_backend_bytes` to detect a complete sequence-0 initial handshake packet.
- [x] Keep malformed/incomplete observed backend bytes non-fatal and non-transitioning.
- [x] Keep `observe_client_bytes` from changing handshake phase.
- [x] Add parser unit tests.
- [x] Add adapter state-transition tests.
- [x] Update backend spec if a new reusable MySQL protocol parser/state convention is established.
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

- Do not store auth plugin challenge bytes.
- Do not log raw handshake payloads.
- Do not implement client handshake response parsing.
- Do not introduce packet buffering in this task.
- Do not emit SQL events from handshake observation.

## Review Gate

Before implementation starts, confirm:

- Initial handshake observation is backend-to-client only.
- Parser exposes safe metadata only.
- Partial packets stay non-fatal until a dedicated buffering task exists.
