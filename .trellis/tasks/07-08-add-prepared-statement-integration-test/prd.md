# Add Prepared Statement Integration Test

## Goal

Implement Issue 060 by proving SQL Lens can capture a real MySQL prepared
statement execution through the Docker-backed live runtime and expose original
SQL, decoded parameters, expanded SQL, and redacted sensitive values through the
REST API.

The user value is end-to-end confidence that prepared statement debugging works
with a real driver, not only parser fixtures.

## Source Issue

Issue 060: Add prepared statement integration test.

Description: Add an integration test proving prepared statement capture and
expansion with a real driver.

Labels: `area:testing`, `area:protocol-mysql`, `type:test`
Priority: P0
Difficulty: Hard
Estimated Time: 8h
Dependencies: Issue 055, Issue 059

## Confirmed Facts

- Issue 055 is archived as
  `07-07-render-expanded-sql-for-prepared-statements`.
- Issue 059 is archived as
  `07-08-add-mysql-live-integration-test-with-docker`.
- Issue 055 intentionally kept expanded SQL on MySQL-local execute envelope
  state and did not emit `SqlEvent` for prepared statement executions.
- Issue 059 added a Docker-gated MySQL live test and minimal app runtime glue
  that forwards through the proxy, invokes `MysqlProtocolAdapter`, stores
  emitted events in shared `ApiState`, and verifies via `GET /api/v1/sql-events`.
- `MysqlConnectionState` currently records prepared statement prepare responses,
  execute envelopes, decoded parameters, and expanded SQL.
- `MysqlProtocolAdapter::observe_backend_bytes` currently emits events only via
  `observe_backend_query_response`; there is no prepared statement execute event
  emission path yet.
- Storage redacts retained events through `RingBufferStore::append`, and API
  list/detail responses read from that store.
- The Issue 059 Docker test uses MySQL 8.0 with
  `mysql_native_password` because the current authentication observer covers
  the simple OK/ERR authentication path.

## Requirements

- R1. Add a Docker-backed integration test that uses a real MySQL driver
  prepared statement path through the SQL Lens proxy.
- R2. Reuse the Issue 059 minimal live runtime path instead of introducing a
  second runtime harness.
- R3. Prepare and execute a parameterized statement with at least one ordinary
  parameter and one sensitive parameter.
- R4. Verify the API exposes a captured prepared statement execute event with:
  original/template SQL, decoded parameters, expanded SQL, protocol `mysql`,
  kind `statement_execute`, and status `ok`.
- R5. Cover redaction for one parameter by choosing a sensitive parameter name
  or SQL pattern that the existing redaction policy masks before storage/API
  exposure.
- R6. Add the minimal MySQL adapter emission needed to turn existing prepared
  execute envelope state into a stored `SqlEvent`.
- R7. Keep the test Docker-gated/skippable like Issue 059.
- R8. Preserve existing COM_QUERY live coverage and workspace tests.
- R9. Do not add UI, replay, persistent storage, broad runtime composition,
  resultset capture, new redaction policy semantics, or non-MySQL behavior.

## Acceptance Criteria

- [x] Docker-backed prepared statement integration test is present.
- [x] Test starts MySQL and SQL Lens using the shared Issue 059 runtime helper.
- [x] Test prepares and executes a parameterized query through the proxy.
- [x] API returns a captured prepared statement execute event.
- [x] API event includes original/template SQL.
- [x] API event includes decoded parameters.
- [x] API event includes expanded SQL.
- [x] API-visible payload proves one sensitive parameter is redacted.
- [x] Test is skipped or clearly gated when Docker is unavailable.
- [x] Existing Issue 059 live query test still passes.
- [x] Existing MySQL protocol unit tests still pass.
- [x] `rtk cargo fmt --check` passes.
- [x] `rtk cargo test --workspace` passes.
- [x] `rtk cargo clippy --workspace --all-targets -- -D warnings` passes.

## Out Of Scope

- Resultset capture for prepared `SELECT` responses unless needed for the
  smallest viable test; prefer an OK-packet statement if it still exercises
  prepared execute parameters and expansion.
- Expanding authentication support beyond the Issue 059 MySQL 8.0 native
  password test setup.
- Persistent storage, replay, UI, WebSocket assertions, or plugin behavior.
- A production CLI runtime that starts all services.

## Scope Decision

This cannot be a test-only change because existing MySQL prepared execute state
does not emit `SqlEvent` yet. The task may add the smallest adapter event
emission path for completed prepared statement executions, reusing existing
decoded parameter and expanded SQL state.
