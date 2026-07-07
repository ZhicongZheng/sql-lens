# Store prepared statement state per connection

## Goal

Implement Issue 049: maintain MySQL prepared statement ID to SQL template mappings scoped to a single `MysqlConnectionState`.

## Background

- Issue 047 parses client `COM_STMT_PREPARE` and stores `MysqlPendingStatementPrepare`.
- Issue 048 parses backend prepare OK/ERR responses and stores `MysqlStatementPrepareOutcome`.
- Successful prepare responses contain the server-assigned MySQL statement ID.
- Later `COM_STMT_EXECUTE` parsing needs a connection-local way to look up the original SQL template by statement ID.
- The protocol adapter API currently has no explicit connection close hook; protocol state is owned per connection and is dropped with that connection state.
- Scope decision: Issue 049 treats connection close cleanup as `MysqlConnectionState` ownership/drop cleanup and does not add a shared protocol close hook.

## Requirements

- Store successful prepared statements in `MysqlConnectionState`.
- Key the map by MySQL server statement ID (`u32`).
- Store the original SQL template from the prepare command.
- Store prepare metadata needed by later execute parsing: parameter count, column count, and optional warning count.
- Keep prepared statement mappings connection-local.
- Do not store failed prepare outcomes in the prepared statement map.
- Replacing an existing statement ID in the same connection should update the mapping with the latest successful prepare outcome.
- Expose a narrow read API for tests and future execute parsing.
- Treat connection close cleanup as state ownership cleanup unless a close hook is added in a separate task.
- Do not add core model fields, storage schema, API, WebSocket, or UI changes.
- Do not parse `COM_STMT_EXECUTE` in this task.
- Do not add new dependencies.

## Acceptance Criteria

- [x] Successful prepare OK inserts a statement mapping into the current `MysqlConnectionState`.
- [x] Statement mapping includes statement ID, SQL template, parameter count, column count, and optional warning count.
- [x] Failed prepare does not insert a statement mapping.
- [x] Reusing a statement ID updates the current connection mapping.
- [x] A second `MysqlConnectionState` does not see mappings from the first state.
- [x] New connection state starts with an empty statement map.
- [x] Existing prepare outcome behavior remains available for debugging/future consumers.
- [x] Existing `COM_QUERY` behavior remains unchanged.
- [x] `cargo fmt --check` passes.
- [x] `cargo test -p sql-lens-protocol-mysql` passes.
- [x] `cargo test --workspace` passes.
- [x] `cargo clippy --workspace --all-targets -- -D warnings` passes.

## Out of Scope

- Explicit protocol adapter close hook.
- `COM_STMT_EXECUTE`, `COM_STMT_CLOSE`, `COM_STMT_RESET`.
- Statement cleanup on close beyond `MysqlConnectionState` drop semantics.
- Storage, API, WebSocket, UI, and plugin exposure.
- Protocol-neutral core model changes.
