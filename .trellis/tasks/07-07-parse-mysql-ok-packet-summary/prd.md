# Parse MySQL OK packet summary

## Goal

Implement Issue 045: decode basic MySQL OK packet fields for affected rows and status, then populate successful `COM_QUERY` events with a protocol-neutral result summary.

## Background

- Issue 044 emits one `SqlEvent` when a pending MySQL `COM_QUERY` is finalized by a backend OK or ERR packet.
- Successful events currently set `CaptureStatus::Ok` but leave `SqlEvent.result` as `None`.
- `sql-lens-core` already exposes `ResultSummary { affected_rows, returned_rows }`.
- Detailed result-set lifecycle parsing is not implemented yet, so row-returning query counts remain out of scope.

## Requirements

- Parse MySQL command OK packet payloads with header `0x00`.
- Decode `affected_rows` from the OK packet length-encoded integer.
- Decode and skip `last_insert_id` so later fields are read from the correct offset.
- Decode `status_flags` when at least two bytes remain after the two length-encoded integers.
- Keep warning count, info string, session tracking fields, and EOF-as-OK behavior out of scope.
- Populate successful finalized `COM_QUERY` events with `ResultSummary { affected_rows: Some(value), returned_rows: None }` when the summary is available.
- Preserve `CaptureStatus::Ok` for successful OK events.
- Store MySQL-only OK status flags under `ProtocolMetadata` when available.
- Treat malformed OK summary parsing as non-fatal in adapter observation.
- Do not add new dependencies.

## Acceptance Criteria

- [x] A fixture MySQL OK packet with `affected_rows = 0` parses successfully.
- [x] A fixture MySQL OK packet with non-zero `affected_rows` parses successfully.
- [x] Length-encoded integer decoding covers at least one-byte and `0xfc` two-byte forms.
- [x] Successful `COM_QUERY` event includes `ResultSummary.affected_rows`.
- [x] Successful `COM_QUERY` event keeps `ResultSummary.returned_rows = None`.
- [x] Successful `COM_QUERY` event metadata includes MySQL OK status flags when present.
- [x] Backend OK observation remains non-fatal when summary fields are malformed.
- [x] Error events remain unchanged and keep `result = None`.
- [x] `cargo fmt --check` passes.
- [x] `cargo test -p sql-lens-protocol-mysql` passes.
- [x] `cargo test --workspace` passes.
- [x] `cargo clippy --workspace --all-targets -- -D warnings` passes.

## Out of Scope

- Full result-set lifecycle parsing.
- Row-returned counts for SELECT result sets.
- EOF packet handling and `0xfe` OK-as-EOF handling.
- Warning count, status info, session state tracking, and last insert ID in public core models.
- Prepared statement execution summaries.
- Storage, API, WebSocket, and UI changes.
