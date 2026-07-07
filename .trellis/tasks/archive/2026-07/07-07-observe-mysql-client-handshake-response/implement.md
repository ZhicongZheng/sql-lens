# Observe MySQL client handshake response plan

## Checklist

- [x] Read backend specs before implementation.
- [x] Add client handshake response safe metadata type.
- [x] Add structured client handshake response parse error type.
- [x] Implement fixed Protocol 41 header parsing.
- [x] Implement NUL-terminated username parsing.
- [x] Implement auth response skipping for secure-connection and length-encoded forms.
- [x] Implement optional database parsing based on capability flags.
- [x] Implement optional auth plugin name parsing based on capability flags.
- [x] Re-export parser contracts from `lib.rs`.
- [x] Extend `MysqlConnectionPhase` with client-handshake-seen phase.
- [x] Extend `MysqlConnectionState` with sanitized client handshake metadata.
- [x] Update `observe_client_bytes` to detect client handshake response only after initial handshake.
- [x] Keep malformed/incomplete observed client bytes non-fatal and non-transitioning.
- [x] Add parser unit tests.
- [x] Add adapter state-transition tests.
- [x] Update backend spec with client handshake response contract.
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

- Do not store auth response bytes.
- Do not log raw handshake response payloads.
- Do not implement TLS/SSLRequest handling.
- Do not detect authentication success or failure.
- Do not emit SQL events from authentication observation.

## Review Gate

Before implementation starts, confirm:

- The parser exposes only safe metadata.
- Client response observation requires the server initial handshake to be seen first.
- Partial packets stay non-fatal until a dedicated buffering task exists.
