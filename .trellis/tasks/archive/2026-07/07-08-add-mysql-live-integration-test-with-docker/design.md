# Design: MySQL Live Integration Test With Docker

## Problem

Issue 059 needs an end-to-end signal:

1. A real MySQL server runs in Docker.
2. SQL Lens accepts a client connection.
3. A simple query is sent through SQL Lens to MySQL.
4. SQL Lens captures the query event.
5. The REST API returns the captured event.

The current codebase has most components, but not the glue:

- `TcpForwarder` forwards bytes without protocol observation.
- `sql-lens-app` does not start proxy or API runtimes.
- API state can expose stored events, but no runtime path writes captured proxy
  events into that state.

## Boundaries

### In Scope

- A minimal test/demo runtime path that combines:
  - TCP proxy listener and backend dialer.
  - MySQL protocol adapter observation.
  - In-memory event store used by the API.
  - HTTP API server with shared `ApiState`.
- A Docker-backed MySQL integration test that exercises that path.
- Test gating for Docker availability.

### Out Of Scope

- Full CLI runtime management.
- Persistent storage.
- Web UI.
- Replay.
- Multi-protocol runtime registry composition beyond MySQL.
- TLS termination.

## Proposed Architecture

Introduce the smallest reusable runtime helper, most likely in a backend crate
that can be used by tests without turning `sql-lens-app` into a large runtime
yet.

Candidate shape:

```rust
pub struct MinimalSqlLensRuntime {
    pub proxy_addr: SocketAddr,
    pub api_addr: SocketAddr,
    shutdown: ...
}
```

The runtime owns:

- `ApiState` with an in-memory `RingBufferStore`.
- HTTP server task bound to `127.0.0.1:0`.
- TCP listener task bound to `127.0.0.1:0`.
- Per-connection proxy tasks that dial the Docker MySQL backend.
- A protocol-observing forwarder for MySQL.

## Protocol-Observing Forwarding

The current `TcpForwarder` uses `copy_bidirectional`, which cannot inspect each
direction. The integration path needs a small loop that:

- Reads bytes from client and backend independently.
- Calls `MysqlProtocolAdapter::observe_client_bytes` for client-to-backend
  chunks.
- Calls `MysqlProtocolAdapter::observe_backend_bytes` for backend-to-client
  chunks.
- Writes emitted events through the shared `ApiState` used by the API server.
- Still forwards bytes immediately; event capture must not block forwarding on
  UI or nonessential work.

For Issue 059, the event sink can be direct and in-memory, but the sharing
boundary is `ApiState` rather than a separate test-only store. A later runtime
task can route through `sql-lens-capture` fan-out if needed.

The MySQL adapter emits completed `COM_QUERY` events while observing backend
responses, so the forwarding loop must observe both directions. Observing only
client-to-backend bytes is not enough to prove the query completed and became a
REST-visible `SqlEvent`.

## API Verification

The test should call:

```text
GET http://<api_addr>/api/v1/sql-events
```

Then assert that at least one item has:

- `protocol = "mysql"`
- `kind = "query"`
- `status = "ok"`
- `original_sql` matching the simple query

## Docker And MySQL Client Strategy

Preferred approach:

- Use a Rust Docker test library such as `testcontainers` to start MySQL.
- Use a Rust MySQL client crate to connect through `proxy_addr` and execute
  `SELECT 1`.

Fallback approach if dependency APIs or sandbox constraints block Docker:

- Keep the integration test gated by an environment variable and skip clearly
  when Docker is unavailable.
- Preserve all non-Docker unit tests.

## Compatibility

- Existing `TcpForwarder` behavior should remain unchanged unless a small,
  separately tested observing forwarder is introduced.
- Existing API response contracts should not change.
- Existing MySQL parser/adapter tests should remain unchanged.
- The new integration test should not require Docker for normal
  `cargo test --workspace` unless the team deliberately chooses always-on Docker
  tests.

## Risks

- Docker may not be available in CI or local sandboxes.
- MySQL startup timing can be flaky without explicit readiness checks.
- MySQL auth plugin defaults can require client compatibility choices.
- Observing arbitrary TCP chunks may expose packet framing assumptions; the
  first integration test should keep the query simple and tolerate packet
  boundaries through existing parser behavior where possible.
