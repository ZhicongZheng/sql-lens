# Design: MySQL-Compatible Compatibility Smoke Tests

## Problem

SQL Lens advertises StarRocks, TiDB, and Apache Doris as MySQL-compatible
targets, but live integration coverage currently proves only MySQL. Issues
061-063 should add opt-in smoke tests that exercise these databases through the
same SQL Lens proxy/API path without turning the project into a broad database
middleware or CI orchestration task.

## Boundaries

### In Scope

- Docker-gated StarRocks, TiDB, and Doris smoke tests.
- Basic connect/text query through SQL Lens for all three targets.
- Prepared statement smoke coverage for TiDB.
- Documentation of Doris prepared statement behavior.
- API verification using existing SQL event response shapes.

### Out Of Scope

- CI workflow scaffolding.
- Production runtime changes.
- Feature-complete database compatibility suites.
- SQL rewrite, resultset capture expansion, replay, UI, or persistent storage.

## Proposed Architecture

Use the existing Issue 059/060 live path:

1. Start the target database in Docker/Testcontainers.
2. Wait until its MySQL-compatible query port accepts a stable readiness query.
3. Start `start_minimal_mysql_runtime` with the target backend address.
4. Connect a MySQL-compatible Rust driver to the SQL Lens proxy.
5. Execute the target smoke query or prepared statement.
6. Poll `GET /api/v1/sql-events` and, when needed, detail responses until the
   captured events appear.

Prefer one file per target under `crates/sql-lens-app/tests/` unless a small
shared helper module is clearly simpler. Keep helper extraction conservative:
only share readiness/API polling if duplication becomes awkward across all
three tests.

## Target Coverage

### StarRocks

- Gate: `SQL_LENS_STARROCKS_TESTS=1`.
- Coverage: connect, text query, API event capture.
- Expected event: protocol `mysql`, kind `query`, status `ok`, original SQL
  matching the smoke query.

### TiDB

- Gate: `SQL_LENS_TIDB_TESTS=1`.
- Coverage: connect, text query, prepared statement execution, API event
  capture.
- Expected text event: same core shape as MySQL live query coverage.
- Expected prepared event: same core shape as MySQL prepared live coverage,
  including `statement_execute`, template SQL, decoded parameters, and expanded
  SQL where TiDB/MySQL-compatible binary protocol behavior allows it.

### Apache Doris

- Gate: `SQL_LENS_DORIS_TESTS=1`.
- Coverage: connect, text query, API event capture.
- Prepared statements: document observed behavior. Do not force prepared
  execution into the first smoke if Doris driver/server behavior makes it
  unstable or unsupported.

## Compatibility Notes To Verify

- StarRocks docs provide a single-container Docker quick start using
  `starrocks/allin1-ubuntu` and exposing query port `9030`.
- TiDB docs describe MySQL-compatible access on port `4000`; Docker Hub exposes
  the `pingcap/tidb` image and versioned tags.
- Doris docs describe a Docker quick-start script for a complete cluster, while
  Docker Hub runtime images are split by component (`fe-*`, `be-*`, etc.).
  Prefer a script/all-in-one route if practical; otherwise keep Doris text-query
  smoke narrow and document any multi-container setup.
- Verify image names, pinned tags, query ports, default credentials, and startup
  commands for StarRocks, TiDB, and Doris during implementation.
- Whether each selected image is single-container or requires a small
  multi-container cluster.
- Whether the MySQL driver authentication path works with the current
  `MysqlProtocolAdapter` observer.
- Stable smoke query choice per database, preferably `SELECT 1` or the smallest
  accepted text query.
- Whether prepared statement support differs for TiDB/Doris and how that appears
  in the API.

## Risks

- StarRocks and Doris containers may be heavier and slower than MySQL/TiDB.
- Some official images may require multi-service orchestration.
- Authentication or handshake differences may expose gaps in the current
  MySQL-compatible observer.
- Docker image tags can drift, so tests should pin explicit tags once verified.
