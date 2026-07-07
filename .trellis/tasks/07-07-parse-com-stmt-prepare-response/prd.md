# Parse COM_STMT_PREPARE response

## Goal

Implement Issue 048: parse backend `COM_STMT_PREPARE` responses so SQL Lens can capture the server-assigned statement ID and prepare metadata after a client prepare command.

## Background

- Issue 047 added client-side `COM_STMT_PREPARE` parsing and `MysqlPendingStatementPrepare`.
- MySQL-compatible servers answer a successful prepare with a `COM_STMT_PREPARE_OK` packet.
- A failed prepare is reported with the existing MySQL `ERR_Packet` shape.
- Statement ID to SQL template mapping belongs to Issue 049; this task should expose the parsed prepare result needed by that later mapping layer.
- Prepared statement execution, parameter definitions, column definitions, and parameter expansion are later tasks.
- Scope decision: Issue 048 stores only the most recent MySQL-local prepare outcome; the per-connection statement map remains Issue 049.

## Requirements

- Parse successful backend prepare responses while a `MysqlPendingStatementPrepare` exists.
- Extract the server-assigned statement ID.
- Extract parameter count and column count.
- Preserve backend response packet sequence ID in MySQL-local state.
- Handle prepare failure responses by parsing the existing MySQL ERR packet summary.
- Consume the pending prepare state on successful OK or ERR response so stale prepares do not leak into later packets.
- Keep unsupported, malformed, incomplete, or invalid prepare responses non-fatal at adapter level.
- Do not build the per-connection statement map in this task.
- Do not parse parameter definition packets or column definition packets in this task.
- Do not emit `SqlEvent` for prepare response parsing in this task.
- Do not add new dependencies.

## Acceptance Criteria

- [x] Parser extracts statement ID from a valid `COM_STMT_PREPARE_OK` fixture.
- [x] Parser extracts parameter count from a valid `COM_STMT_PREPARE_OK` fixture.
- [x] Parser extracts column count from a valid `COM_STMT_PREPARE_OK` fixture.
- [x] Parser rejects incomplete prepare OK payloads with structured errors.
- [x] Parser recognizes prepare ERR responses using the existing ERR packet summary.
- [x] Adapter consumes pending prepare state after a successful prepare OK response.
- [x] Adapter stores a MySQL-local last prepare outcome containing the original command, response sequence ID, statement ID, parameter count, and column count.
- [x] Adapter consumes pending prepare state after a prepare ERR response and stores the parsed error summary.
- [x] Backend prepare responses without pending prepare state remain non-fatal and emit zero events.
- [x] Malformed prepare responses keep pending prepare state for later complete packets.
- [x] Existing `COM_QUERY` response behavior remains unchanged.
- [x] `cargo fmt --check` passes.
- [x] `cargo test -p sql-lens-protocol-mysql` passes.
- [x] `cargo test --workspace` passes.
- [x] `cargo clippy --workspace --all-targets -- -D warnings` passes.

## Out of Scope

- Per-connection prepared statement map.
- Statement cleanup on connection close.
- `COM_STMT_EXECUTE`, `COM_STMT_CLOSE`, `COM_STMT_RESET`.
- Parameter definition packet parsing.
- Column definition packet parsing.
- Parameter expansion and replay.
- Event emission, storage, API, WebSocket, UI, and plugin changes.
