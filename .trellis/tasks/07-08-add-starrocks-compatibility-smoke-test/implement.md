# Implementation Plan: MySQL-Compatible Compatibility Smoke Tests

## Ordered Steps

1. Confirm Docker startup contracts.
   - Verify official or project-standard image names/tags.
   - Identify exposed MySQL-compatible query ports, default credentials, and
     readiness queries for StarRocks, TiDB, and Doris.
   - Record compatibility notes in the task as they are discovered.

2. Decide helper shape conservatively.
   - Reuse `start_minimal_mysql_runtime`.
   - Extract shared test helpers only if three target tests would otherwise
     duplicate substantial readiness/API polling code.

3. Add StarRocks smoke coverage.
   - Gate on `SQL_LENS_STARROCKS_TESTS=1`.
   - Start StarRocks via Docker/Testcontainers.
   - Connect through SQL Lens proxy.
   - Run a stable text query.
   - Assert captured API query event.

4. Add TiDB smoke coverage.
   - Gate on `SQL_LENS_TIDB_TESTS=1`.
   - Start TiDB via Docker/Testcontainers.
   - Connect through SQL Lens proxy.
   - Run a stable text query and assert API query event.
   - Run a prepared statement and assert API `statement_execute` event with
     template SQL, parameters, and expanded SQL when available.
   - Compare assertions to existing MySQL live query/prepared behavior.

5. Add Doris smoke coverage.
   - Gate on `SQL_LENS_DORIS_TESTS=1`.
   - Start Doris via Docker/Testcontainers.
   - Connect through SQL Lens proxy.
   - Run a stable text query and assert API query event.
   - Document prepared statement behavior and follow-up gaps.

6. Validate.
   - `rtk cargo fmt --check`
   - `rtk cargo test -p sql-lens-app`
   - `rtk proxy env SQL_LENS_STARROCKS_TESTS=1 cargo test -p sql-lens-app --test starrocks_live_docker -- --nocapture`
   - `rtk proxy env SQL_LENS_TIDB_TESTS=1 cargo test -p sql-lens-app --test tidb_live_docker -- --nocapture`
   - `rtk proxy env SQL_LENS_DORIS_TESTS=1 cargo test -p sql-lens-app --test doris_live_docker -- --nocapture`
   - Existing MySQL-gated command if shared helpers changed:
     `rtk proxy env SQL_LENS_DOCKER_TESTS=1 cargo test -p sql-lens-app --test mysql_live_docker -- --nocapture`
   - `rtk cargo test --workspace`
   - `rtk cargo clippy --workspace --all-targets -- -D warnings`

## Candidate Files

- `crates/sql-lens-app/tests/starrocks_live_docker.rs`
- `crates/sql-lens-app/tests/tidb_live_docker.rs`
- `crates/sql-lens-app/tests/doris_live_docker.rs`
- `crates/sql-lens-app/tests/mysql_live_docker.rs` only if shared helper
  extraction is necessary.
- `crates/sql-lens-app/src/lib.rs` only if the minimal runtime needs a tiny,
  backward-compatible database-type/config hook.
- `.trellis/tasks/07-08-add-starrocks-compatibility-smoke-test/*`
- Optional docs/spec file for compatibility notes if implementation discovers a
  durable project convention.

## Rollback Points

- Keep all compatibility tests opt-in so default workspace tests remain fast and
  stable.
- Avoid broad runtime composition changes.
- If a database requires multi-service orchestration that cannot fit a smoke
  test cleanly, document the blocker and split a follow-up harness task rather
  than overbuilding this one.

## Validation Results

- `rtk cargo fmt --check` passed.
- `rtk cargo test -p sql-lens-app` passed.
- `rtk proxy env SQL_LENS_STARROCKS_TESTS=1 cargo test -p sql-lens-app --test starrocks_live_docker -- --nocapture` passed.
- `rtk proxy env SQL_LENS_TIDB_TESTS=1 cargo test -p sql-lens-app --test tidb_live_docker -- --nocapture` passed.
- `rtk proxy env SQL_LENS_DORIS_TESTS=1 cargo test -p sql-lens-app --test doris_live_docker -- --nocapture` passed.
- `rtk proxy env SQL_LENS_DOCKER_TESTS=1 cargo test -p sql-lens-app --test mysql_live_docker -- --nocapture` passed.
- `rtk cargo test --workspace` passed.
- `rtk cargo clippy --workspace --all-targets -- -D warnings` passed.
