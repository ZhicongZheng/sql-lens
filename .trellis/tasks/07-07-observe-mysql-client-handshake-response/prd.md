# Observe MySQL client handshake response

## Goal

Implement Issue 041: observe the MySQL-compatible client handshake response, capture safe user/database metadata, and avoid storing authentication response bytes.

## Background

- Issue 040 observes the backend initial handshake and stores safe setup metadata.
- `PROTOCOL.md` requires authentication traffic to be forwarded while recording state transitions only.
- The client handshake response is client-to-backend traffic and may contain password/authentication response bytes.
- Later Issue 042 will detect authentication success or failure from backend packets.

## Requirements

- Decode a complete MySQL client handshake response packet after the initial server handshake has been seen.
- Capture only safe metadata:
  - client capability flags,
  - max packet size,
  - character set,
  - username when present and valid UTF-8,
  - requested database when present and valid UTF-8,
  - authentication plugin name when present and valid UTF-8.
- Do not store, log, or expose authentication response bytes.
- Move connection phase from `InitialHandshakeSeen` to a client-handshake-observed phase after successful decode.
- Keep malformed or incomplete client response bytes non-fatal until a future packet-buffering task exists.
- Emit no SQL events from client handshake observation.
- Keep parser and state changes inside `sql-lens-protocol-mysql`.

## Acceptance Criteria

- [x] A valid client handshake response payload parses successfully.
- [x] Parsed metadata includes client capability flags, max packet size, character set, and username.
- [x] Requested database is captured only when the client capability flags indicate it is present.
- [x] Authentication plugin name is captured only when the client capability flags indicate it is present.
- [x] Authentication response bytes are skipped and not exposed by the parsed response type.
- [x] Malformed or incomplete client response payloads return structured parse errors in parser tests.
- [x] Observing a complete client handshake response after `InitialHandshakeSeen` updates connection state.
- [x] Observing client handshake-shaped bytes before initial handshake does not update client handshake state.
- [x] The adapter emits no `SqlEvent` records for the client handshake response.
- [x] `cargo fmt --check` passes.
- [x] `cargo test -p sql-lens-protocol-mysql` passes.
- [x] `cargo test --workspace` passes.
- [x] `cargo clippy --workspace --all-targets -- -D warnings` passes.

## Out of Scope

- Authentication success/failure detection.
- Password/authentication response decoding.
- TLS negotiation or SSLRequest handling.
- Packet stream buffering or TCP segmentation handling.
- Command parsing.
- Capture event emission.
- Updating core `ConnectionInfo` or API connection responses.
