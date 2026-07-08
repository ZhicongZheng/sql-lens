# Design: Prepared Statement Integration Test

## Problem

Issue 060 needs an end-to-end prepared statement signal:

1. A real MySQL server runs in Docker.
2. SQL Lens accepts a driver connection through the proxy.
3. The driver prepares and executes a parameterized statement.
4. SQL Lens emits a prepared statement execution event.
5. The REST API returns original SQL, parameters, expanded SQL, and redacted
   sensitive values.

Current code has most parsing pieces, but not the final emission:

- Prepared statement prepare responses are observed and stored in MySQL adapter
  state.
- `COM_STMT_EXECUTE` envelopes can decode common parameter values and render
  expanded SQL.
- `observe_backend_bytes` only emits completed COM_QUERY events today.
- Issue 059 added a minimal runtime path that stores emitted adapter events in
  shared `ApiState`.

## Boundaries

### In Scope

- Minimal prepared statement execute event emission in `sql-lens-protocol-mysql`.
- Reuse of Issue 059 live runtime and Docker-gated MySQL setup.
- API verification through existing SQL event list/detail DTOs.
- Redaction proof through existing storage redaction behavior.

### Out Of Scope

- Resultset capture expansion.
- New redaction policy semantics.
- New production CLI runtime behavior.
- Non-MySQL integrations.

## Proposed Architecture

Extend the MySQL adapter with a prepared statement execute completion path:

- Client bytes:
  - Observe `COM_STMT_PREPARE` and store pending prepare command.
  - Observe `COM_STMT_EXECUTE`, decode parameters, and store
    `MysqlStatementExecuteEnvelope`.
- Backend bytes:
  - Existing prepare response observation continues to populate prepared
    statement metadata.
  - Add an execute response observer that detects OK/ERR terminal packets while
    `last_statement_execute_envelope` is present.
  - Emit a `SqlEvent` with `kind = StatementExecute`.

The emitted event should reuse:

- template SQL from the prepared statement metadata as `original_sql`;
- decoded parameters mapped into `SqlParameter`;
- `expanded_sql` from `MysqlStatementExecuteEnvelope`;
- existing OK/ERR parsers for status, rows, and error summary;
- protocol metadata fields for command and statement ID.

## Test Strategy

Reuse the Issue 059 Docker integration style:

- Gate on `SQL_LENS_DOCKER_TESTS`.
- Start MySQL 8.0 with `mysql_native_password`.
- Wait for direct MySQL readiness.
- Start the minimal SQL Lens runtime against the container.
- Connect the driver through the proxy.
- Prepare a statement with an OK-packet completion path, such as an `INSERT`
  into a temporary table or another parameterized statement that avoids
  resultset parsing.
- Execute it with at least one normal value and one sensitive value.
- Poll the API until a `statement_execute` event appears.

## Redaction Strategy

Use existing storage redaction rather than changing redaction policy behavior.
If the default policy redacts by SQL pattern or parameter name, choose the test
SQL/parameter shape to trigger that policy. If named parameters are not
available in the MySQL binary protocol event model, use a SQL pattern or a value
that existing policy masks before retention.

## Compatibility

- Existing COM_QUERY event emission must keep working.
- Existing prepared statement parser tests should remain valid.
- API response shapes should not change unless an existing field already
  represents the required data.
- Docker integration remains opt-in.

## Risks

- Real drivers may send prepared statements using resultset-returning queries;
  the first integration should use an OK-packet statement to stay inside current
  terminal response support.
- Parameter name redaction may be unavailable for positional MySQL binary
  parameters, requiring SQL-pattern redaction for the sensitive value proof.
- TCP chunk boundaries may expose packet-framing assumptions; keep the first
  live test simple and add packet buffering only if the Docker test proves it is
  necessary.
