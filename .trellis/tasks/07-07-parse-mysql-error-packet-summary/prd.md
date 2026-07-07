# Parse MySQL error packet summary

## Goal

Implement Issue 046: decode MySQL ERR packets into protocol-neutral error summaries for failed `COM_QUERY` events.

## Background

- Issue 044 emits one `SqlEvent` when a pending MySQL `COM_QUERY` is finalized by a backend OK or ERR packet.
- Error events currently set `CaptureStatus::Error` but leave `SqlEvent.error` as `None`.
- Authentication ERR parsing already decodes vendor error code, SQLSTATE, and message for auth state, but query events need a command ERR summary attached to `SqlEvent.error`.
- Error messages are untrusted database text. They may be useful for debugging, but must never be logged by parser or adapter code.

## Requirements

- Parse MySQL ERR packet payloads with header `0xff`.
- Decode the 2-byte little-endian MySQL error code.
- Decode SQLSTATE when the SQL state marker `#` and five SQLSTATE bytes are present.
- Decode the remaining bytes as a human-readable error message.
- Use lossy UTF-8 conversion for the error message so malformed database bytes do not fail packet observation.
- Sanitize the error message for control characters.
- Populate failed finalized `COM_QUERY` events with `ErrorSummary`.
- Store MySQL-only error code details in protocol metadata on the error summary.
- Preserve `CaptureStatus::Error`.
- Treat malformed ERR summary parsing as non-fatal in adapter observation.
- Do not add new dependencies.

## Acceptance Criteria

- [x] A fixture MySQL ERR packet with error code, SQLSTATE, and message parses successfully.
- [x] Error code is captured.
- [x] SQLSTATE is captured when present.
- [x] Error message is decoded and sanitized.
- [x] Failed `COM_QUERY` event includes `ErrorSummary`.
- [x] Error summary metadata includes MySQL error code.
- [x] Backend ERR observation remains non-fatal when summary fields are malformed.
- [x] OK events remain unchanged by ERR summary parsing.
- [x] No parser or adapter code logs raw error messages.
- [x] `cargo fmt --check` passes.
- [x] `cargo test -p sql-lens-protocol-mysql` passes.
- [x] `cargo test --workspace` passes.
- [x] `cargo clippy --workspace --all-targets -- -D warnings` passes.

## Out of Scope

- General-purpose SQL/PII redaction rules.
- Mapping MySQL vendor error codes to semantic categories.
- Authentication flow behavior changes.
- Storage, API, WebSocket, UI, and replay changes.
- PostgreSQL or other protocol error formats.
