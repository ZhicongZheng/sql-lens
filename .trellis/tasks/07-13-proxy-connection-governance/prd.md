# Enforce Proxy Connection Governance And Graceful Shutdown

## Goal

Make configured connection limits, idle timeouts, and graceful shutdown behavior real runtime guarantees for the TCP proxy.

## Confirmed Facts

- `ProxyConfig` defines `max_connections`, `idle_timeout_ms`, and `shutdown_timeout_ms`, but app runtime does not use them: `crates/sql-lens-config/src/model.rs:56-64`.
- Accepted backend connections are spawned without being tracked: `crates/sql-lens-app/src/lib.rs:691-697`.
- Runtime shutdown waits for listener tasks but not active connection tasks: `crates/sql-lens-app/src/lib.rs:246-270`.
- `sql-lens-proxy` already contains `ActiveSessionDrain` and `ProxyShutdownConfig` primitives.

## Requirements

- Enforce the configured maximum number of active proxy sessions without blocking packet forwarding.
- Apply the configured idle timeout to both client/backend session activity.
- Track active session tasks and connection lifecycle state until completion.
- On shutdown, stop accepting clients, drain active sessions up to `shutdown_timeout_ms`, then abort/report timed-out sessions.
- Preserve connection records and final capture events for completed and failed sessions.

## Acceptance Criteria

- A runtime configured with `max_connections = N` accepts at most N active sessions and records rejected clients without panicking.
- An idle session is closed after the configured timeout and its lifecycle is finalized.
- Shutdown returns within the configured drain timeout even with a deliberately blocked session.
- Active sessions that finish before the deadline are classified as completed, and timed-out sessions are reported.
- Existing proxy and live integration tests remain green.

## Out Of Scope

- TLS termination or upstream TLS.
- Authentication or authorization.
- Rate limiting beyond the active-session cap.
