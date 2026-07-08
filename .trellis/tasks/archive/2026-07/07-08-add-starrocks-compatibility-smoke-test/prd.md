# Add MySQL-Compatible Compatibility Smoke Tests

## Goal

Implement Issues 061, 062, and 063 in one Trellis task by adding Docker-only,
environment-gated compatibility smoke tests for StarRocks, TiDB, and Apache
Doris through the SQL Lens MySQL-compatible proxy path.

The user value is early confidence that SQL Lens's MySQL-compatible protocol
family is not MySQL-only in practice. Each target should prove the basic
connect/query path through SQL Lens and expose captured events through the REST
API; TiDB should also prove prepared statement behavior against the Issue 060
prepared execute path.

## Source Issues

Issue 061: Add StarRocks compatibility smoke test.

- Description: Add a compatibility smoke test for StarRocks when running the
  extended matrix.
- Acceptance: separate from default CI, basic connect/query path, known gaps
  documented.
- Labels: `area:compatibility`, `type:test`
- Priority: P2
- Dependencies: Issue 059

Issue 062: Add TiDB compatibility smoke test.

- Description: Add a compatibility smoke test for TiDB.
- Acceptance: connect, text query, prepared query, compare results to MySQL
  behavior, gaps documented.
- Labels: `area:compatibility`, `type:test`
- Priority: P2
- Dependencies: Issue 060

Issue 063: Add Doris compatibility smoke test.

- Description: Add a compatibility smoke test for Apache Doris.
- Acceptance: connect and text query, prepared statement behavior documented,
  gaps tracked as follow-up issues.
- Labels: `area:compatibility`, `type:test`
- Priority: P2
- Dependencies: Issue 059

## Confirmed Facts

- Issue 059 added a Docker-gated MySQL live capture path in
  `crates/sql-lens-app/tests/mysql_live_docker.rs`.
- Issue 060 added MySQL prepared statement execute event emission and a Docker
  live prepared statement capture/redaction test.
- The minimal runtime helper in `crates/sql-lens-app/src/lib.rs` forwards
  MySQL-compatible traffic through `MysqlProtocolAdapter` and stores captured
  events in shared `ApiState`.
- `README.md` states that the first production target is the MySQL-compatible
  protocol family, including MySQL, StarRocks, TiDB, and Apache Doris.
- `GET /api/v1/protocols` reports protocol `mysql` as supported for databases
  `mysql`, `starrocks`, `tidb`, and `doris`.
- `sql-lens-config` already has database type enum variants for StarRocks, TiDB,
  and Doris.
- The repository currently has no `.github/` workflow files and no existing CI
  extended matrix implementation.
- User decision: this combined task should be Docker-only and environment-gated;
  do not add CI workflow scaffolding.

## Requirements

- R1. Add opt-in Docker-backed compatibility smoke tests for StarRocks, TiDB,
  and Apache Doris.
- R2. Keep all compatibility smoke tests separate from default test and CI
  execution through explicit environment variables.
- R3. Reuse the existing minimal runtime path unless a tiny shared test helper
  or runtime parameter is necessary.
- R4. Connect to each target through the SQL Lens proxy using a
  MySQL-compatible Rust driver.
- R5. For StarRocks, cover basic connect and text query capture through the API.
- R6. For TiDB, cover connect, text query capture, prepared query capture, and
  compare the observed behavior to the existing MySQL live-test expectations.
- R7. For Apache Doris, cover basic connect and text query capture through the
  API, and document prepared statement behavior as supported, unsupported, or
  not exercised by the first smoke.
- R8. Document known compatibility gaps discovered while implementing or
  running the smoke tests.
- R9. Preserve existing MySQL live Docker tests and default workspace tests.

## Acceptance Criteria

- [x] StarRocks compatibility smoke test is present.
- [x] TiDB compatibility smoke test is present.
- [x] Apache Doris compatibility smoke test is present.
- [x] Each test is Docker-only and skipped unless its environment gate is set.
- [x] StarRocks test covers connect, text query, and API event capture.
- [x] TiDB test covers connect, text query, prepared query, and API event
      capture for both query kinds.
- [x] TiDB results/observed API payloads are compared to MySQL live-test
      behavior where applicable.
- [x] Doris test covers connect, text query, and API event capture.
- [x] Doris prepared statement behavior is documented.
- [x] Known compatibility gaps are documented in the task or project docs.
- [x] Existing MySQL live Docker tests still pass.
- [x] `rtk cargo fmt --check` passes.
- [x] `rtk cargo test --workspace` passes.
- [x] `rtk cargo clippy --workspace --all-targets -- -D warnings` passes.

## Out Of Scope

- CI workflow scaffolding or matrix job definitions.
- Full feature coverage for StarRocks, TiDB, or Doris.
- Performance, benchmark, replication, cluster-management, or HA testing.
- Resultset capture work beyond what is needed for smoke-test assertions.
- Persistent storage, UI, replay, or non-MySQL protocol behavior.

## Planning Status

Scope decision is resolved: Docker-only and environment-gated. No product-scope
question remains before implementation.

## Compatibility Notes

- StarRocks and Doris are exercised through the MySQL-compatible text query
  path only in this first smoke. Prepared statement coverage remains TiDB plus
  the existing MySQL live test.
- `mysql_async` must be configured with `prefer_socket=false` for these Docker
  smoke tests. Without it, the client may query `@@socket` when connecting to
  localhost-style mapped ports; StarRocks rejects that MySQL-specific system
  variable.
- Doris startup is materially slower than MySQL, StarRocks, and TiDB in the
  selected all-in-one image, so its smoke test uses a longer opt-in readiness
  window and remains outside default tests.
