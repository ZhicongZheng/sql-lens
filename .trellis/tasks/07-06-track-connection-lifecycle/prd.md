# Track connection lifecycle

## Goal

Issue 017: record proxy connection lifecycle states from accept through backend dial, forwarding close, and backend dial failure.

## User Value

SQL Lens needs a connection timeline before SQL capture exists. Developers should be able to reason about whether a client was accepted, backend dialing succeeded or failed, forwarding closed normally, or the session failed.

## Background

- `sql-lens-core` already defines protocol-neutral `ConnectionId`, `ConnectionInfo`, and `ConnectionState`.
- `sql-lens-proxy` already has:
  - `AcceptedClient`
  - `BackendDialer`
  - `BackendDialFailure`
  - `ForwardingSummary`
  - `ForwardingFailure`
  - `ShutdownDrainSummary`
- `ARCHITECTURE.md` requires stable connection IDs and connection records even if handshake fails.
- This task should not add storage persistence. Storage integration comes later.

## Requirements

- Add proxy-local connection lifecycle tracking.
- Generate a stable connection ID for each accepted session.
- Track client address and backend address.
- Track connection state transitions aligned with current architecture:
  - created/accepted
  - backend connected
  - closing
  - closed
  - failed
- Track byte counters from forwarding summaries.
- Track backend dial failure as a failed connection lifecycle.
- Keep lifecycle records protocol-neutral.
- Reuse `sql-lens-core` connection types where practical.
- Do not add `uuid`, `time`, `chrono`, storage crates, API crates, protocol crates, or app runtime wiring in this task.
- Add unit tests for normal close and backend dial failure.

## Out Of Scope

- Persistent storage.
- REST/API exposure.
- WebSocket broadcasting.
- Protocol handshake states beyond already-defined core variants.
- User/database extraction.
- Query counts from SQL capture.
- App runtime session orchestration.
- UUID/time crate integration.
- Multi-process stable IDs.

## Acceptance Criteria

- [ ] A connection ID is generated for each lifecycle record.
- [ ] Lifecycle records use protocol-neutral core types.
- [ ] Created/accepted state is represented.
- [ ] Backend connected state is represented.
- [ ] Normal forwarding close transitions to closed.
- [ ] Backend dial failure transitions to failed.
- [ ] Byte counters update from `ForwardingSummary`.
- [ ] Backend dial failure context maps into lifecycle failure state.
- [ ] Unit tests cover normal close.
- [ ] Unit tests cover backend dial failure.
- [ ] `cargo fmt --check` passes.
- [ ] `cargo check --workspace` passes.
- [ ] `cargo test --workspace` passes.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes.
- [ ] No storage, API, app runtime, protocol parsing, UUID/time dependency, or capture pipeline is introduced.

## Open Questions

None blocking.

## Notes

- Keep `prd.md` focused on requirements, constraints, and acceptance criteria.
- Lightweight tasks can remain PRD-only.
- For complex tasks, add `design.md` for technical design and `implement.md` for execution planning before `task.py start`.
