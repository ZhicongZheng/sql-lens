# Observe MySQL initial handshake

## Goal

Implement Issue 040: decode enough of the MySQL-compatible server initial handshake packet to identify protocol setup and move the MySQL adapter connection state forward.

## Background

- Issue 037 added `sql-lens-protocol-mysql` and a minimal `MysqlProtocolAdapter`.
- Issue 038 added MySQL packet envelope parsing.
- Issue 039 added packet framing fixtures.
- `PROTOCOL.md` defines the connection-level state machine with `HandshakeSeen` before client authentication.
- MySQL-compatible servers send the initial handshake from backend to client as the first protocol packet after TCP connection establishment.

## Requirements

- Decode a MySQL-compatible initial server handshake packet from backend bytes.
- Detect the packet only when the MySQL connection state is waiting for the initial handshake.
- Move connection state from awaiting initial handshake to handshake seen after successful decode.
- Store only non-sensitive handshake metadata needed for later authentication and diagnostics:
  - protocol version,
  - server version,
  - connection ID,
  - capability flags when present,
  - character set when present,
  - status flags when present,
  - authentication plugin name when present.
- Do not store or log authentication challenge bytes / scramble data.
- Keep observation protocol-neutral at the shared adapter boundary.
- Keep forwarding observation non-blocking and event-free for this task.
- Add focused unit tests for handshake parsing and adapter state transition.

## Acceptance Criteria

- [x] A valid initial server handshake payload parses successfully.
- [x] Parsed handshake metadata includes protocol version, server version, connection ID, and optional capability/status fields.
- [x] Authentication challenge bytes are skipped and not exposed on the parsed handshake type.
- [x] Malformed or incomplete handshake payloads return structured parse errors in parser tests.
- [x] `MysqlConnectionState` starts in an awaiting-initial-handshake phase.
- [x] Observing a complete backend handshake packet moves state to handshake seen.
- [x] Observing client bytes does not move state to handshake seen.
- [x] The adapter emits no `SqlEvent` records for the initial handshake.
- [x] No passwords, authentication responses, or challenge bytes are logged or persisted.
- [x] `cargo fmt --check` passes.
- [x] `cargo test -p sql-lens-protocol-mysql` passes.
- [x] `cargo test --workspace` passes.
- [x] `cargo clippy --workspace --all-targets -- -D warnings` passes.

## Out of Scope

- Client handshake response parsing.
- Authentication result detection.
- TLS negotiation.
- Packet stream buffering or TCP segmentation handling.
- Multi-packet reassembly.
- SQL command parsing.
- Capture event emission.
- Logging protocol payloads.
