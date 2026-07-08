# Add MySQL Live Integration Test With Docker

## Goal

Implement Issue 059 by proving SQL Lens can capture a real MySQL query in a
Docker-backed integration test and expose the captured SQL event through the
REST API.

The user value is a runnable end-to-end confidence check: start MySQL, start a
minimal SQL Lens runtime, run a query through the proxy, then verify the API
returns the captured event.

## Source Issue

Issue 059: Add MySQL live integration test with Docker.

Description: Add a Docker-based integration test for MySQL through SQL Lens.

Labels: `area:testing`, `area:protocol-mysql`, `type:test`
Priority: P0
Difficulty: Hard
Estimated Time: 8h
Dependencies: Issue 028, Issue 044

## Confirmed Facts

- Issue 028 is archived as `07-07-implement-sql-event-list-endpoint`.
- Issue 044 is archived as `07-07-capture-com-query-timing`.
- `sql-lens-api` exposes `router_with_state`, `bind_http_server`, and
  `GET /api/v1/sql-events`.
- `ApiState` owns in-memory `RingBufferStore`, `ConnectionStore`, and
  `LiveStatistics`.
- `sql-lens-proxy` currently provides listener, backend dialer, and raw
  bidirectional TCP forwarding.
- `TcpForwarder` currently uses `tokio::io::copy_bidirectional` and does not
  observe protocol packets or emit capture events.
- `sql-lens-app` currently reads config, validates config, initializes logging,
  emits a startup check, and exits. It does not start proxy or API runtimes.
- `sql-lens-protocol-mysql` can observe MySQL handshakes, authentication,
  `COM_QUERY`, prepared statement commands, `COM_PING`, and `COM_QUIT`, and can
  emit a completed `SqlEvent` for terminal `COM_QUERY` responses.
- The workspace does not currently include Docker/testcontainers or a MySQL
  Rust client dependency.
- Context7 documentation tooling was not available in this session; dependency
  APIs should be checked before final implementation if new crates are added.

## Requirements

- R1. Add a Docker-backed integration test that starts a real MySQL-compatible
  server.
- R2. Start a minimal SQL Lens runtime in the test.
- R3. Run a simple query through the SQL Lens proxy, not directly against MySQL.
- R4. Verify the SQL Lens API returns a captured `SqlEvent` for that query.
- R5. Keep the test opt-in or clearly skippable when Docker is unavailable.
- R6. Keep runtime glue minimal and scoped to the Issue 059 demo path: proxy
  forwarding may call the MySQL adapter and write captured events into the
  shared `ApiState`, but must not become full production app composition.
- R7. Do not add persistent storage, UI, replay behavior, SQL rewrite, auth, TLS
  termination, or non-MySQL protocol behavior.
- R8. Preserve existing unit and workspace tests.

## Acceptance Criteria

- [x] Docker-backed MySQL integration test is present.
- [x] Test starts MySQL and waits until it is ready.
- [x] Test starts a SQL Lens proxy listener and API server using the same
      `ApiState`.
- [x] Proxy forwarding invokes the MySQL adapter while bytes flow through the
      proxy and stores emitted events through the shared `ApiState`.
- [x] Test runs `SELECT 1` or equivalent through the proxy.
- [x] Test calls `GET /api/v1/sql-events` and finds the captured query event.
- [x] Test is skipped or clearly gated when Docker is unavailable.
- [x] Existing app CLI smoke tests still pass.
- [x] Existing proxy/API/protocol unit tests still pass.
- [x] `rtk cargo fmt --check` passes.
- [x] `rtk cargo test --workspace` passes.
- [x] `rtk cargo clippy --workspace --all-targets -- -D warnings` passes.

Implementation note: the Docker integration uses `DO 1` as the equivalent
simple proxy query because current MySQL adapter query-event emission is
validated for OK/ERR COM_QUERY terminal packets, while `SELECT 1` returns a
resultset flow that belongs to later resultset capture work.

## Out Of Scope

- Full production `sql-lens-app` runtime composition beyond the minimal demo
  needs.
- Web frontend.
- Persistent SQLite/DuckDB storage.
- Replay API or replay UI.
- MySQL TLS/auth plugin expansion beyond what the selected test driver/server
  requires.
- Prepared statement integration coverage; Issue 060 owns that.
- PostgreSQL, ClickHouse, SQLite, or other database integration tests.

## Scope Decision

The repository currently cannot satisfy the Issue 059 acceptance criteria with a
test-only change because raw proxy forwarding does not observe packets and the
app does not start proxy/API runtimes.

Decision: include the smallest runtime glue required for the test: an internal
test/demo path that forwards bytes while invoking the MySQL protocol adapter and
writes emitted events into the shared `ApiState` used by the REST API. This
keeps Issue 059 meaningful without pretending the production app is fully
composed.
