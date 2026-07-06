# Implement Backend Dialing

## Goal

Implement Issue 014: connect accepted client sessions to the configured backend database address with a bounded dial timeout and structured dial failure reporting.

## User Value

After SQL Lens accepts a client connection, it must open the matching upstream database connection before byte forwarding can exist. This task creates that second network leg while keeping forwarding, protocol parsing, and lifecycle persistence for later tasks.

## Background

- Issue 013 is complete: `sql-lens-proxy` can bind a TCP listener, accept client connections, and shut down the accept loop.
- `AcceptedClient` currently owns the client `TcpStream` and peer address.
- `ARCHITECTURE.md` defines the proxy lifecycle step `DialingBackend` after `AcceptingConnection`.
- `CONFIG.md` / `sql-lens-config` define:
  - `backend.address`
  - `proxy.connect_timeout_ms`
- Issue 015 will implement bidirectional TCP forwarding.
- Issue 017 will add durable connection lifecycle records. This task should not invent the final lifecycle model.
- Context7 confirmed Tokio supports `TcpStream::connect` and `tokio::time::timeout`.

## Requirements

- Add backend dialing support to `sql-lens-proxy`.
- Define a small `BackendDialConfig` with backend address and connect timeout.
- Provide a conversion path from existing config structs so backend address and timeout come from runtime configuration.
- Connect a previously accepted client to the configured backend address.
- Enforce `proxy.connect_timeout_ms` as the backend dial timeout.
- Return a successful paired connection object containing:
  - accepted client stream
  - backend stream
  - client peer address
  - backend address string
- Return structured dial failures that preserve:
  - backend address
  - client peer address
  - timeout duration when timeout happens
  - source `std::io::Error` when connect fails
- Provide a lightweight failure record type that later lifecycle work can consume.
- Add async tests for successful dial, refused/failed dial, and timeout behavior where practical.

## Dependency Policy

- Reuse `tokio` already present in `sql-lens-proxy`.
- Reuse `tracing` already present in `sql-lens-proxy` for low-sensitivity dial lifecycle logs if useful.
- Allow adding a path dependency from `sql-lens-proxy` to `sql-lens-config` only for typed config conversion.
- Do not add `thiserror`, `anyhow`, `async-trait`, database client libraries, protocol crates, or forwarding utilities in this task.

## Out Of Scope

- Wiring backend dialing into `sql-lens-app`.
- Making the CLI long-running.
- Bidirectional byte forwarding.
- Byte counters.
- Protocol parsing.
- Authentication handling.
- Connection IDs.
- Persistent connection lifecycle records.
- Capture pipeline events.
- TLS to backend.
- Retry policy.
- Backend connection pooling.
- DNS caching.

## Acceptance Criteria

- [ ] `sql-lens-proxy` exposes a backend dialing API.
- [ ] Backend address is sourced from existing config structs.
- [ ] Backend connect timeout is sourced from existing config structs.
- [ ] Successful dial returns a paired client/backend connection object.
- [ ] Dial timeout is enforced.
- [ ] Dial failures return structured errors.
- [ ] Dial failures expose a lightweight failure record.
- [ ] Tests cover successful backend dial.
- [ ] Tests cover failed backend dial.
- [ ] Tests cover timeout or document why deterministic timeout is not practical.
- [ ] `cargo fmt --check` passes.
- [ ] `cargo check --workspace` passes.
- [ ] `cargo test --workspace` passes.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes.
- [ ] No byte forwarding, protocol parsing, capture, app runtime startup, or signal handling is introduced.

## Open Questions

None blocking.
