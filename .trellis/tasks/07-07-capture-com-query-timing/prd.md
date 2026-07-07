# Capture COM_QUERY timing

## Goal

Implement Issue 044: measure duration from a parsed MySQL `COM_QUERY` command to a backend terminal OK/ERR response and emit a completed `SqlEvent` with duration.

## Background

- Issue 043 parses `COM_QUERY` client command payloads after authentication.
- `SqlEvent` already has `duration` and `timings` fields.
- `ProtocolAdapter::observe_*_bytes` receives a `CaptureEventEmitter`, so the MySQL adapter can emit normalized events without depending on the capture channel crate.
- Detailed OK packet result summaries and detailed ERR packet decoding are later issues.

## Requirements

- Start a pending query timing record when a valid `COM_QUERY` is observed after authentication.
- Finalize the pending query when the backend returns a terminal OK packet.
- Finalize the pending query when the backend returns a terminal ERR packet.
- Emit exactly one `SqlEvent` per finalized query.
- Record duration in `SqlEvent.duration` and `SqlEvent.timings.duration`.
- Preserve SQL text from the pending `COM_QUERY`.
- Populate protocol-neutral event fields from `ProtocolConnectionContext`.
- Include MySQL command metadata under `ProtocolMetadata`.
- Keep detailed OK result summary decoding out of scope.
- Keep detailed ERR packet summary decoding out of scope, but mark the event as error.
- Keep unsupported backend responses non-fatal and keep the pending query open.

## Acceptance Criteria

- [x] `COM_QUERY` starts a pending query timing record.
- [x] Backend OK after pending `COM_QUERY` emits one `SqlEvent` with `CaptureStatus::Ok`.
- [x] Backend ERR after pending `COM_QUERY` emits one `SqlEvent` with `CaptureStatus::Error`.
- [x] Emitted events include SQL text, duration, timing fields, connection context, and MySQL metadata.
- [x] Unsupported backend responses do not finalize or emit an event.
- [x] Backend terminal response without pending query does not emit an event.
- [x] Success and error path tests cover duration and event count.
- [x] `cargo fmt --check` passes.
- [x] `cargo test -p sql-lens-protocol-mysql` passes.
- [x] `cargo test --workspace` passes.
- [x] `cargo clippy --workspace --all-targets -- -D warnings` passes.

## Out of Scope

- Parsing affected rows or OK status flags.
- Parsing detailed MySQL ERR packet metadata into protocol-neutral `ErrorSummary`.
- Result-set lifecycle parsing.
- Prepared statement timing.
- SQL normalization, fingerprinting, redaction, or parameter expansion.
- Storage, REST API, WebSocket fan-out, and UI changes.
