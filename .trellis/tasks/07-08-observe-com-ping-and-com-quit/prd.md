# Observe COM_PING and COM_QUIT

## Goal

Implement Issue 058 by observing MySQL `COM_PING` and `COM_QUIT` client
commands as connection activity.

This task should update MySQL-local connection activity/state without storing
ping as SQL and without adding storage/API/UI behavior.

## Source Issue

Issue 058: Observe COM_PING and COM_QUIT.

Description: Track ping and quit commands as connection activity.

Labels: `area:protocol-mysql`, `area:proxy`, `type:feature`
Priority: P1
Difficulty: Easy
Estimated Time: 3h
Dependencies: Issue 042

## Confirmed Facts

- `ConnectionInfo` already has `last_activity_at`, `state`, and `query_count`
  fields.
- `ConnectionState` already has `Closing` and `Closed` variants.
- `MysqlConnectionState` owns a `ConnectionInfo`, but currently exposes
  command-specific state rather than a read-only connection accessor.
- `MysqlCommandKind` currently has SQL-oriented command kinds plus
  `StatementClose`.
- Existing `COM_QUERY` timing emits SQL events; ping and quit should not become
  SQL events in this task.
- Client command observation only runs after the MySQL phase is
  `Authenticated`.

## Requirements

- R1. Parse MySQL `COM_PING` client command payloads.
- R2. Parse MySQL `COM_QUIT` client command payloads.
- R3. `COM_PING` must update connection `last_activity_at` using the
  observation clock.
- R4. `COM_QUIT` must update connection `last_activity_at` and move connection
  state toward `ConnectionState::Closing`.
- R5. Ping and quit must not start a pending SQL query.
- R6. Ping and quit must not emit `SqlEvent`s by default.
- R7. Ping and quit must not mutate prepared statement maps or execute
  envelopes.
- R8. Expose enough read-only MySQL connection state for focused tests without
  exposing mutable internal state.

## Acceptance Criteria

- [ ] Parser tests cover `COM_PING`.
- [ ] Parser tests cover `COM_QUIT`.
- [ ] Adapter tests prove ping updates `last_activity_at`.
- [ ] Adapter tests prove quit moves connection state to `Closing`.
- [ ] Adapter tests prove ping is not stored as SQL and emits no SQL event.
- [ ] Existing query, prepare, execute, and close tests continue to pass.
- [ ] `rtk cargo fmt --check` passes.
- [ ] `rtk cargo test -p sql-lens-protocol-mysql` passes.
- [ ] `rtk cargo test --workspace` passes.
- [ ] `rtk cargo clippy --workspace --all-targets -- -D warnings` passes.

## Out Of Scope

- Emitting ping or quit events to storage/API/WebSocket/UI.
- Closing TCP sockets or changing proxy forwarding behavior.
- Parsing backend OK packets for ping.
- Updating shared connection storage.
- Authentication, TLS, or handshake behavior.
- PostgreSQL or other protocol ping/quit equivalents.

## Implementation Notes

- Keep parsing in `sql-lens-protocol-mysql/src/command.rs`.
- Use the existing observation clock for deterministic activity timestamps in
  tests.
- `COM_PING` and `COM_QUIT` have command bytes only; any richer handling should
  be left to a later task.

## Notes

- PRD-only is sufficient because this is a narrow MySQL protocol-state task.
