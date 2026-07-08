# Implement COM_STMT_CLOSE cleanup

## Goal

Implement Issue 057 by observing MySQL `COM_STMT_CLOSE` commands and removing
the matching prepared statement from per-connection MySQL state.

This is a protocol-state cleanup task, not an event-emission task. Closing a
statement should update MySQL-local prepared statement state, keep unknown
statement closes harmless, and avoid adding storage/API/UI behavior.

## Source Issue

Issue 057: Implement COM_STMT_CLOSE cleanup.

Description: Parse close commands and remove statement state.

Labels: `area:protocol-mysql`, `type:feature`
Priority: P1
Difficulty: Medium
Estimated Time: 4h
Dependencies: Issue 049

## Confirmed Facts

- `MysqlConnectionState` stores prepared statements in
  `prepared_statements: BTreeMap<u32, MysqlPreparedStatement>`.
- `MysqlConnectionState::prepared_statement(statement_id)` and
  `prepared_statement_count()` already expose state for tests.
- `MysqlCommandKind` already has `StatementClose` in the protocol-neutral event
  enum, but MySQL command parsing currently handles query, prepare, and
  execute paths.
- `COM_STMT_CLOSE` is client-to-server only and has no backend OK/ERR response.
- Existing prepared statement tasks intentionally left close cleanup for this
  issue.

## Requirements

- R1. Parse MySQL `COM_STMT_CLOSE` client command payloads.
- R2. Extract the little-endian statement ID from the close command.
- R3. Remove a known statement ID from `MysqlConnectionState` immediately when
  the close command is observed after authentication.
- R4. Closing an unknown statement ID must be harmless.
- R5. Incomplete or malformed close payloads must not mutate state.
- R6. Close cleanup must not emit SQL events in this issue.
- R7. Close cleanup must not affect pending query timing, pending prepare
  response handling, or statement execute envelopes.
- R8. Do not add storage, API, WebSocket, UI, replay, or redaction behavior in
  this task.

## Acceptance Criteria

- [ ] A known prepared statement is removed after observing `COM_STMT_CLOSE`.
- [ ] Closing an unknown statement does not panic and does not change existing
      statement state.
- [ ] Malformed or incomplete close packets do not mutate state.
- [ ] Tests cover parse-level close command behavior.
- [ ] Tests cover adapter/state-level cleanup behavior.
- [ ] Existing prepare and execute tests continue to pass.
- [ ] `rtk cargo fmt --check` passes.
- [ ] `rtk cargo test -p sql-lens-protocol-mysql` passes.
- [ ] `rtk cargo test --workspace` passes.
- [ ] `rtk cargo clippy --workspace --all-targets -- -D warnings` passes.

## Out Of Scope

- Emitting `SqlEventKind::StatementClose` events.
- Recording close commands in storage, API, WebSocket, or UI.
- Releasing server resources or forwarding changes beyond observing the packet.
- Prepared statement replay behavior.
- PostgreSQL or other protocol close/deallocate behavior.

## Implementation Notes

- Keep parsing MySQL-local in `sql-lens-protocol-mysql`.
- Preserve the existing state-machine behavior: only observe commands after the
  connection is authenticated.
- `COM_STMT_CLOSE` has no backend terminal response, so cleanup should happen
  during client command observation.

## Notes

- PRD-only is sufficient for this task because the implementation is limited to
  one protocol crate and has narrow, testable behavior.
