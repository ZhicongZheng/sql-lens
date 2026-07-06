# Implement bidirectional TCP forwarding

## Goal

Issue 015: forward bytes between paired client/backend TCP streams, track byte counters, and end the session cleanly when either side closes.

## User Value

After SQL Lens accepts a client and dials the configured backend, it must become a transparent TCP proxy. This task makes the proxy useful for raw connectivity before protocol parsing and SQL capture exist.

## Background

- Issue 013 completed the TCP listener and accepted client handoff.
- Issue 014 completed backend dialing and returns `ProxiedConnection`.
- Tokio documentation confirms `tokio::io::copy_bidirectional`:
  - copies both directions concurrently,
  - returns `(bytes_from_a_to_b, bytes_from_b_to_a)`,
  - shuts down the opposite writer after EOF on one side,
  - completes after both directions shut down,
  - requires Tokio `io-util`.
- Future tasks will add graceful shutdown, connection lifecycle persistence, protocol parsing, capture events, and byte-counter storage.

## Requirements

- Add bidirectional TCP forwarding support to `sql-lens-proxy`.
- Consume a `ProxiedConnection` and forward bytes between the client stream and backend stream.
- Track byte counters for:
  - client to backend
  - backend to client
- Return a structured forwarding summary on clean completion.
- Return structured forwarding errors that preserve:
  - client peer address
  - backend address
  - partial byte counters when Tokio provides them or when they are known
  - source `std::io::Error`
- Either side closing must end the forwarding session cleanly after Tokio finishes shutting down both directions.
- Keep forwarding independent from protocol parsing and capture.
- Add async tests that verify bytes pass in both directions and counters are correct.

## Dependency Policy

- Reuse `tokio` in `sql-lens-proxy`.
- Allow enabling Tokio `io-util` for `tokio::io::copy_bidirectional`.
- Reuse `tracing` only for low-sensitivity lifecycle logs.
- Do not add `thiserror`, `anyhow`, `tokio-util`, protocol crates, storage crates, app crates, database clients, TLS libraries, or retry utilities.

## Out Of Scope

- Wiring sessions into `sql-lens-app`.
- Running a long-lived proxy service.
- Graceful shutdown of active forwarding sessions.
- Connection IDs.
- Persistent connection lifecycle records.
- Storage or statistics integration.
- SQL capture.
- Protocol parsing.
- TLS.
- Backpressure policy beyond Tokio's copy behavior.
- Replay.

## Acceptance Criteria

- [ ] `sql-lens-proxy` exposes a forwarding API that consumes `ProxiedConnection`.
- [ ] Forwarding copies client bytes to backend.
- [ ] Forwarding copies backend bytes to client.
- [ ] Forwarding returns client-to-backend byte count.
- [ ] Forwarding returns backend-to-client byte count.
- [ ] Either side closing ends the forwarding session cleanly.
- [ ] Forwarding errors are structured and expose connection context.
- [ ] Tests cover bidirectional copy behavior.
- [ ] Tests cover byte counters.
- [ ] Tests cover clean close behavior.
- [ ] `cargo fmt --check` passes.
- [ ] `cargo check --workspace` passes.
- [ ] `cargo test --workspace` passes.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes.
- [ ] No protocol parsing, capture pipeline, storage write, app startup, or signal handling is introduced.

## Open Questions

None blocking.

## Notes

- Keep `prd.md` focused on requirements, constraints, and acceptance criteria.
- Lightweight tasks can remain PRD-only.
- For complex tasks, add `design.md` for technical design and `implement.md` for execution planning before `task.py start`.
