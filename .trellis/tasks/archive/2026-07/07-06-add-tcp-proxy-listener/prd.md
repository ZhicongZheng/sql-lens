# Add TCP Proxy Listener

## Goal

Implement Issue 013: add the first TCP proxy listener in `sql-lens-proxy`.

The listener should bind the configured proxy address, accept client TCP connections, expose structured errors, and support shutting down the accept loop.

## User Value

SQL Lens needs a reliable network entry point before backend dialing, forwarding, protocol observation, and capture can exist. This task creates that first proxy boundary while keeping later backend/session work cleanly separable.

## Background

- `sql-lens-proxy` currently exists as an empty crate.
- `ARCHITECTURE.md` assigns TCP listener, session lifecycle, forwarding, shutdown, and backpressure coordination to `sql-lens-proxy`.
- `CONFIG.md` defines `proxy.listen` as the database proxy listener address.
- `sql-lens-config::ProxyConfig` already validates that `proxy.listen` is not empty, but it intentionally does not probe socket bind availability.
- `ARCHITECTURE.md` recommends Tokio for the Rust async runtime and one task per accepted proxy connection.
- Context7 confirmed Tokio supports `TcpListener::bind`, `TcpListener::accept`, `tokio::select!`, `oneshot` / channel shutdown patterns, and `#[tokio::test]`.
- Issue 014 will add backend dialing. Issue 015 will add bidirectional forwarding. Issue 016 will add full graceful shutdown and draining.

## Requirements

- Add a Tokio-based TCP listener implementation to `sql-lens-proxy`.
- Provide a small runtime listener configuration type owned by `sql-lens-proxy`.
- Bind to the configured listen address string.
- Expose the actual local address after binding, including OS-assigned ports such as `127.0.0.1:0`.
- Return structured bind errors that preserve the listen address and source `std::io::Error`.
- Accept client TCP connections.
- Represent accepted client connections with peer address and owned client stream.
- Provide an accept loop that can be stopped by a shutdown signal.
- Return accept loop statistics such as accepted connection count.
- Keep backend dialing, forwarding, protocol parsing, capture, and app runtime integration out of scope.
- Add focused async unit tests for bind, bind failure, accept, and shutdown.

## Dependency Policy

- Allow adding `tokio` to `sql-lens-proxy` with only the features needed for this task:
  - `net`
  - `sync`
  - `time`
  - `rt`
  - `macros`
- Allow adding `tracing` to `sql-lens-proxy` for low-sensitivity listener lifecycle events if useful.
- Do not add `tokio-util`, `thiserror`, `anyhow`, `async-trait`, or runtime framework dependencies.
- Do not add dependencies to `sql-lens-core` for this task.

## Out Of Scope

- Wiring the listener into `sql-lens-app`.
- Making the `sql-lens` binary long-running.
- Signal handling.
- Backend dialing.
- Bidirectional forwarding.
- Connection IDs and lifecycle records.
- Capture pipeline events.
- Protocol adapter integration.
- TLS handling.
- Max connection enforcement.
- Idle timeouts.
- Graceful draining of active sessions.

## Acceptance Criteria

- [x] `sql-lens-proxy` exposes a TCP listener API.
- [x] Listener binds to a configured address.
- [x] Listener exposes its bound local address.
- [x] Bind failures return a structured error containing the requested listen address.
- [x] Accepted client connections include peer address and owned client stream.
- [x] Accept loop can deliver accepted connections through an internal channel boundary.
- [x] Accept loop can be shut down without accepting a connection.
- [x] Tests cover successful bind.
- [x] Tests cover bind failure.
- [x] Tests cover accepting a client connection.
- [x] Tests cover shutting down the accept loop.
- [x] `cargo fmt --check` passes.
- [x] `cargo check --workspace` passes.
- [x] `cargo test --workspace` passes.
- [x] `cargo clippy --workspace --all-targets -- -D warnings` passes.
- [x] No backend dialing, byte forwarding, protocol parsing, capture, API, app runtime startup, or signal handling is introduced.

## Open Questions

None blocking.
