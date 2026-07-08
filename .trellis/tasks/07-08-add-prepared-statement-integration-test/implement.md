# Implementation Plan: Prepared Statement Integration Test

## Ordered Steps

1. Confirm current prepared statement event gap.
   - Use codegraph and existing tests to identify where `COM_STMT_EXECUTE`
     state is captured and where backend responses should emit events.
   - Preserve Issue 055 expanded SQL behavior.

2. Add minimal prepared execute event emission.
   - Add a MySQL adapter helper that converts `MysqlStatementExecuteEnvelope`
     plus OK/ERR response into `SqlEventKind::StatementExecute`.
   - Map decoded parameters into `SqlParameter`.
   - Preserve original/template SQL and expanded SQL.
   - Keep malformed or unsupported bytes non-fatal.

3. Add focused protocol tests.
   - Assert prepared execute emits a `StatementExecute` event for an OK
     response.
   - Assert parameters and expanded SQL are included.
   - Assert existing COM_QUERY emission still passes.

4. Add Docker-backed integration coverage.
   - Reuse `start_minimal_mysql_runtime`.
   - Reuse the Issue 059 MySQL 8.0/native password setup.
   - Execute a prepared parameterized OK-packet statement through the proxy.
   - Poll API list/detail until the statement execute event appears.
   - Assert original SQL, decoded parameters, expanded SQL, and redaction.

5. Validate.
   - `rtk cargo fmt --check`
   - `rtk cargo test -p sql-lens-protocol-mysql`
   - `rtk cargo test -p sql-lens-app`
   - Docker-gated integration command:
     `SQL_LENS_DOCKER_TESTS=1 cargo test -p sql-lens-app --test mysql_live_docker -- --nocapture`
   - `rtk cargo test --workspace`
   - `rtk cargo clippy --workspace --all-targets -- -D warnings`

## Candidate Files

- `crates/sql-lens-protocol-mysql/src/lib.rs`
- `crates/sql-lens-protocol-mysql/src/execute.rs`
- `crates/sql-lens-app/tests/mysql_live_docker.rs`
- `crates/sql-lens-app/src/lib.rs` only if the runtime helper needs a tiny
  reusable extension.
- `.trellis/tasks/07-08-add-prepared-statement-integration-test/*`

## Rollback Points

- Keep event emission isolated from existing COM_QUERY logic.
- Keep Docker test additions inside the existing opt-in test file or a sibling
  opt-in file.
- If redaction cannot be proven through default storage policy without changing
  semantics, stop and revisit scope before modifying redaction rules.

## Completion Notes

- Added prepared execute event emission in the MySQL adapter when an OK/ERR
  backend terminal packet completes a stored `COM_STMT_EXECUTE` envelope.
- Reused existing `SqlEvent` fields, protocol metadata, ring-buffer storage, and
  default redaction policy; no API schema changes were required.
- Added a Docker-gated live test that prepares and executes a MySQL driver
  statement through the proxy, then verifies the captured `statement_execute`
  event through API summary/detail responses.
- Validation completed:
  - `rtk cargo fmt --check`
  - `rtk cargo test -p sql-lens-protocol-mysql`
  - `rtk cargo test -p sql-lens-app`
  - `rtk proxy env SQL_LENS_DOCKER_TESTS=1 cargo test -p sql-lens-app --test mysql_live_docker -- --nocapture`
  - `rtk cargo test --workspace`
  - `rtk cargo clippy --workspace --all-targets -- -D warnings`
