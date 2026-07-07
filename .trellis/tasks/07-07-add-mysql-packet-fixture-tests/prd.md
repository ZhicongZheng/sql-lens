# Add MySQL packet fixture tests

## Goal

Implement Issue 039: add golden packet fixtures for MySQL packet framing so packet parser behavior is documented and regression-tested with reusable fixture files.

## Requirements

- Add fixture files for MySQL packet framing.
- Fixtures must include:
  - normal packet with non-empty payload,
  - empty-payload packet,
  - malformed short-header packet,
  - malformed incomplete-payload packet.
- Document the fixture format.
- Tests must load fixture files and assert parsed payload length and sequence ID for valid packets.
- Tests must assert graceful parse errors for malformed fixtures.
- Do not add payload parsing, handshake parsing, or command parsing.

## Acceptance Criteria

- [x] Normal fixture parses with expected payload length and sequence ID.
- [x] Empty-payload fixture parses with payload length `0` and expected sequence ID.
- [x] Malformed short-header fixture returns `IncompleteHeader`.
- [x] Malformed incomplete-payload fixture returns `IncompletePayload`.
- [x] Fixture format is documented.
- [x] `cargo fmt --check` passes.
- [x] `cargo test --workspace` passes.
- [x] `cargo clippy --workspace --all-targets -- -D warnings` passes.

## Out of Scope

- Live database captures.
- Binary fixture decoding beyond simple hex bytes.
- Packet stream reassembly.
- MySQL payload parsing.
