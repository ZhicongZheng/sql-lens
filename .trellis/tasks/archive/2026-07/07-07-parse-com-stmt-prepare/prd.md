# Parse COM_STMT_PREPARE

## Goal

Implement Issue 047: parse MySQL `COM_STMT_PREPARE` client command packets and store a pending prepared-statement template record.

## Background

- SQL Lens currently parses `COM_QUERY` after authentication and can emit completed query events.
- Prepared statement support starts with the client prepare command.
- MySQL `COM_STMT_PREPARE` uses command byte `0x16`; the remaining payload is the SQL template string.
- The backend prepare response that contains `statement_id`, parameter count, and column count is a later task.

## Requirements

- Parse client command payloads with command byte `0x16` as `COM_STMT_PREPARE`.
- Extract the SQL template string after the command byte.
- Preserve packet sequence ID in MySQL-specific command metadata.
- Add a MySQL connection-state pending statement-prepare record after authentication.
- Keep `COM_STMT_PREPARE` before authentication non-fatal and ignored.
- Keep invalid UTF-8 SQL template bytes non-fatal at adapter level.
- Keep unsupported command bytes non-fatal.
- Do not emit `SqlEvent` for prepare command parsing in this task.
- Do not parse backend prepare response in this task.
- Do not add new dependencies.

## Acceptance Criteria

- [x] Parser extracts SQL template from a valid `COM_STMT_PREPARE` fixture.
- [x] Parser accepts an empty SQL template.
- [x] Parser rejects invalid UTF-8 template bytes with a structured error.
- [x] Adapter stores pending statement-prepare state after authentication.
- [x] Pending statement-prepare state includes command kind, sequence ID, and SQL template.
- [x] `COM_STMT_PREPARE` before authentication does not update state.
- [x] Unsupported command behavior remains unchanged.
- [x] `COM_QUERY` behavior remains unchanged.
- [x] `cargo fmt --check` passes.
- [x] `cargo test -p sql-lens-protocol-mysql` passes.
- [x] `cargo test --workspace` passes.
- [x] `cargo clippy --workspace --all-targets -- -D warnings` passes.

## Out of Scope

- Backend `COM_STMT_PREPARE_OK` response parsing.
- Statement ID allocation or mapping.
- Parameter definition packets.
- Prepared statement execute/close/reset.
- Parameter expansion.
- Event emission, storage, API, WebSocket, and UI changes.
