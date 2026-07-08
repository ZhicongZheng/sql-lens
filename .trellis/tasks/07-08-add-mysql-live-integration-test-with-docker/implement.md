# Implementation Plan: MySQL Live Integration Test With Docker

## Ordered Steps

1. [x] Confirm dependency choices.
   - Check current official docs for the selected Docker test library and MySQL
     client crate before editing.
   - Prefer widely used crates with async Tokio support.

2. [x] Add minimal runtime glue.
   - Add only the dependencies required by the selected runtime/test path.
   - Keep `sql-lens-app` changes small unless the runtime helper naturally
     belongs there.
   - Expose a helper suitable for integration tests: proxy address, API address,
     and graceful shutdown handle.
   - The helper must create one shared `ApiState` and pass it to both the API
     server and the proxy event sink.

3. [x] Add protocol-observing forwarding for MySQL.
   - Keep existing raw `TcpForwarder` behavior intact.
   - Add focused tests for emitted events being written to an in-memory sink.
   - Ensure forwarding continues after observation.
   - Observe client-to-backend bytes for commands and backend-to-client bytes
     for terminal query responses.
   - Store emitted events through the shared `ApiState` so API verification
     exercises the same state boundary as the runtime helper.

4. [x] Add Docker-backed MySQL integration test.
   - Start MySQL container.
   - Wait until MySQL accepts connections.
   - Start minimal SQL Lens runtime with backend address set to the container.
   - Connect a MySQL client through the SQL Lens proxy.
   - Execute a simple query such as `SELECT 1`.
   - Poll `GET /api/v1/sql-events` until the captured event appears or timeout.

5. [x] Gate Docker availability.
   - The default workspace test suite should remain usable on machines without
     Docker unless the user explicitly chooses always-on Docker tests.
   - Skipped tests should print a clear reason.

6. [x] Update docs/specs if the runtime glue creates a reusable convention.

## Candidate Files

- `Cargo.toml`
- `crates/sql-lens-app/Cargo.toml`
- `crates/sql-lens-app/src/main.rs`
- `crates/sql-lens-proxy/src/forwarding.rs`
- `crates/sql-lens-proxy/src/lib.rs`
- `crates/sql-lens-api/src/server.rs`
- `crates/sql-lens-app/tests/`
- Possible new integration test crate or root `tests/` directory if that is
  cleaner for cross-crate runtime testing.

## Validation Commands

- `rtk cargo fmt --check`
- `rtk cargo test -p sql-lens-proxy`
- `rtk cargo test -p sql-lens-api`
- `rtk cargo test -p sql-lens-protocol-mysql`
- `rtk cargo test -p sql-lens-app`
- Docker-gated integration test command chosen during implementation.
- `rtk cargo test --workspace`
- `rtk cargo clippy --workspace --all-targets -- -D warnings`

## Rollback Points

- Dependency additions: keep them isolated so they can be reverted cleanly if
  Docker/client APIs do not fit.
- Runtime glue: keep it separate from existing raw forwarder behavior.
- Integration test: keep Docker gating explicit so normal tests remain stable.

## Scope Decision

Issue 059 includes the minimal runtime glue required to satisfy its own
acceptance criteria. Without that, the test could start MySQL and perhaps
exercise lower-level parser code, but it could not honestly prove SQL Lens
captures a query through the proxy and exposes it through the API.
