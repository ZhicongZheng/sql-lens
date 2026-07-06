# Add proxy graceful shutdown

## Goal

Issue 016: stop accepting new proxy connections, notify active sessions, and drain forwarding sessions with a configurable shutdown timeout.

## User Value

When SQL Lens shuts down, it should stop accepting new database clients and give already-started proxy sessions a bounded chance to finish instead of dropping everything abruptly. This makes the proxy safer to use during local development and prepares the runtime for later CLI/service composition.

## Background

- Issue 013 added `TcpProxyListener::run_accept_loop` with a `watch::Receiver<bool>` shutdown signal.
- Issue 014 added backend dialing.
- Issue 015 added `TcpForwarder::forward(ProxiedConnection)` and byte counters.
- `sql-lens-config::ProxyConfig` currently has `connect_timeout_ms` and `idle_timeout_ms`, but no dedicated shutdown drain timeout.
- `ARCHITECTURE.md` describes shutdown as stopping accepts, notifying sessions, and draining work within a timeout.
- Later tasks will add app runtime startup, OS signal handling, connection lifecycle records, and capture channel draining.

## Requirements

- Add a configurable proxy shutdown timeout to `sql-lens-config`.
- Keep shutdown timeout parsing/defaulting in config models; do not start services from config.
- Add a proxy-local graceful shutdown/session drain API in `sql-lens-proxy`.
- Reuse the existing listener shutdown signal instead of inventing a second listener mechanism.
- Provide a way to notify active sessions that shutdown was requested.
- Provide a bounded drain operation that waits for active session tasks up to the configured shutdown timeout.
- Return structured shutdown/drain results that report:
  - completed sessions
  - timed out sessions
  - failed sessions when join/forwarding errors are observable
- Add unit tests for config default/TOML parsing.
- Add async tests for listener stop, session notification, successful drain, and drain timeout.

## Dependency Policy

- Reuse `tokio` already present in `sql-lens-proxy`.
- Reuse `sql-lens-config` from proxy only for typed config conversion if needed.
- Do not add `tokio-util`, `thiserror`, `anyhow`, app crates, protocol crates, storage crates, signal handling crates, or database clients.

## Out Of Scope

- OS signal handling.
- Wiring shutdown into `sql-lens-app`.
- Starting the long-running proxy service.
- Connection IDs.
- Durable connection lifecycle records.
- Capture channel draining.
- Protocol parsing.
- Storage writes.
- TLS shutdown semantics.
- Retry policy.

## Acceptance Criteria

- [ ] `ProxyConfig` has a default `shutdown_timeout_ms`.
- [ ] TOML config can override `proxy.shutdown_timeout_ms`.
- [ ] Proxy shutdown config can be converted to a `Duration`.
- [ ] Listener shutdown still stops accepting new connections.
- [ ] Active sessions can receive shutdown notification.
- [ ] Session drain waits for active sessions that finish before timeout.
- [ ] Session drain reports timeout when active sessions do not finish before timeout.
- [ ] Drain results are structured and testable.
- [ ] Tests cover config default and TOML override.
- [ ] Tests cover session notification.
- [ ] Tests cover successful drain.
- [ ] Tests cover drain timeout.
- [ ] `cargo fmt --check` passes.
- [ ] `cargo check --workspace` passes.
- [ ] `cargo test --workspace` passes.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes.
- [ ] No app startup, OS signal handling, protocol parsing, capture, storage, or connection lifecycle persistence is introduced.

## Open Questions

None blocking.

## Notes

- Keep `prd.md` focused on requirements, constraints, and acceptance criteria.
- Lightweight tasks can remain PRD-only.
- For complex tasks, add `design.md` for technical design and `implement.md` for execution planning before `task.py start`.
