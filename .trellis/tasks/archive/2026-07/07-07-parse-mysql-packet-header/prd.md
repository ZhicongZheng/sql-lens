# Parse MySQL packet header

## Goal

Implement Issue 038: add MySQL-compatible packet envelope header parsing in `sql-lens-protocol-mysql`.

The parser should read the 4-byte packet header used by the MySQL client/server protocol so later tasks can frame handshake, command, and response payloads safely.

## Background

- Issue 037 added a minimal `MysqlProtocolAdapter` and `MysqlConnectionState`.
- MySQL-compatible packet headers use:
  - 3 little-endian bytes for payload length.
  - 1 byte for sequence ID.
- The payload length does not include the 4-byte packet header.

## Requirements

- Add a small packet module under `sql-lens-protocol-mysql`.
- Parse the 3-byte payload length as a little-endian unsigned integer.
- Parse the 1-byte sequence ID.
- Return the remaining payload bytes separately from the parsed header.
- Reject inputs shorter than 4 bytes gracefully.
- Reject inputs that declare a payload length larger than the available payload bytes gracefully.
- Keep the parser allocation-free for successful parsing.
- Do not parse MySQL payload contents yet.
- Do not integrate packet parsing into `MysqlProtocolAdapter` byte observation yet unless trivial and testable without changing behavior.

## Acceptance Criteria

- [x] Parser returns payload length for normal packets.
- [x] Parser returns sequence ID.
- [x] Parser returns a payload slice matching the declared length.
- [x] Parser rejects buffers shorter than the 4-byte header.
- [x] Parser rejects incomplete payloads.
- [x] Unit tests cover normal, empty-payload, short-header, and incomplete-payload cases.
- [x] `cargo fmt --check` passes.
- [x] `cargo test --workspace` passes.
- [x] `cargo clippy --workspace --all-targets -- -D warnings` passes.

## Out of Scope

- Multi-packet reassembly.
- MySQL handshake parsing.
- Command parsing.
- Prepared statement parsing.
- Capture event emission.
- Packet fixture files; Issue 039 owns golden fixtures.
