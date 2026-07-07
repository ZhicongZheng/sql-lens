# Detect MySQL authentication result

## Goal

Implement Issue 042: detect MySQL authentication success or failure from backend packets and update MySQL connection state.

## Background

- Issue 040 observes the server initial handshake.
- Issue 041 observes the client handshake response and moves the phase to `ClientHandshakeSeen`.
- `PROTOCOL.md` requires authentication traffic to be forwarded while recording state transitions only.
- After the client handshake response, the backend can return OK, ERR, auth switch, or other authentication continuation packets.

## Requirements

- Observe backend authentication result packets only after a client handshake response has been seen.
- Detect authentication success from a MySQL OK packet.
- Detect authentication failure from a MySQL ERR packet.
- Move connection state to an authenticated phase on OK.
- Move connection state to an authentication-failed phase on ERR.
- Store only safe auth result metadata:
  - success/failure status,
  - vendor error code and SQL state when present,
  - sanitized error message when present.
- Keep unsupported authentication continuation packets non-fatal and non-transitioning.
- Emit no SQL events from authentication result observation.
- Do not parse SQL commands in this task.

## Acceptance Criteria

- [x] Backend OK packet after `ClientHandshakeSeen` marks the MySQL connection authenticated.
- [x] Backend ERR packet after `ClientHandshakeSeen` marks the MySQL connection authentication failed.
- [x] Authentication failure metadata captures error code, SQL state, and message when present.
- [x] Backend auth-result-shaped packets before client handshake do not update auth state.
- [x] Unsupported auth continuation packets stay non-fatal and non-transitioning.
- [x] The adapter emits no `SqlEvent` records for authentication result observation.
- [x] `cargo fmt --check` passes.
- [x] `cargo test -p sql-lens-protocol-mysql` passes.
- [x] `cargo test --workspace` passes.
- [x] `cargo clippy --workspace --all-targets -- -D warnings` passes.

## Out of Scope

- Auth switch request parsing.
- Authentication continuation handling.
- TLS handling.
- Command parsing.
- SQL event emission.
- Updating shared `ConnectionInfo` or API connection responses.
- Persisting raw backend authentication packet payloads.
