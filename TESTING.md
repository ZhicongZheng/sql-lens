# SQL Lens Testing Strategy

## Goals

Testing must prove that SQL Lens forwards traffic correctly, captures SQL accurately, protects sensitive data, and remains fast under development workloads.

## Test Layers

### Unit Tests

Targets:

- Capture event models.
- SQL parameter rendering.
- Redaction rules.
- Ring buffer behavior.
- Config parsing.
- API schema serialization.
- Plugin hook dispatch.

Rules:

- Keep tests deterministic.
- Avoid network dependencies in unit tests.
- Include boundary cases for empty SQL, large SQL, binary parameters, and unknown protocol metadata.

### Protocol Tests

Targets:

- MySQL packet framing.
- Sequence ID handling.
- Handshake state transitions.
- Authentication state transitions.
- `COM_QUERY`.
- `COM_STMT_PREPARE`.
- `COM_STMT_EXECUTE`.
- `COM_STMT_CLOSE`.
- Error packets.

Fixtures:

- Raw packet bytes.
- Driver-generated sessions.
- Golden expected capture events.

Protocol tests must not rely only on live databases. Golden packet fixtures are required for reproducibility.

### Integration Tests

Use Docker Compose for real database compatibility.

Initial services:

- MySQL.
- StarRocks.
- TiDB.
- Apache Doris.

Test flow:

1. Start database.
2. Start SQL Lens.
3. Connect using a normal database client.
4. Run text queries.
5. Run prepared statements.
6. Verify API events.
7. Verify WebSocket events.
8. Verify no secret values are logged.

### Compatibility Tests

Drivers to cover:

- Rust `mysql_async`.
- Rust `sqlx` MySQL.
- Go `go-sql-driver/mysql`.
- Java MySQL Connector/J.
- Node.js `mysql2`.
- Python `mysqlclient` or `PyMySQL`.

Each driver should cover:

- Connect.
- Select.
- Insert.
- Prepared select.
- Prepared insert.
- Error query.
- Transaction.

### Frontend Tests

Targets:

- Dashboard rendering.
- SQL list filtering.
- SQL detail rendering.
- Parameter table.
- Monaco SQL display.
- ECharts charts.
- Settings forms.
- WebSocket live update handling.

Recommended tools:

- Vitest.
- React Testing Library.
- Playwright.

### API Tests

Targets:

- REST filters.
- Pagination.
- Error responses.
- WebSocket subscription filters.

### Security Tests

Targets:

- Password redaction.
- SQL redaction.
- XSS escaping for SQL text and error messages.
- Replay confirmation.
- Plugin timeout and failure isolation.

### Benchmark Tests

Targets:

- Proxy overhead.
- QPS.
- Memory usage.
- CPU usage.
- Capture event drop behavior.
- Ring buffer read/write performance.

See [BENCHMARK.md](BENCHMARK.md).

## Docker Compose Plan

Planned compose services:

- `mysql`.
- `starrocks`.
- `tidb`.
- `doris`.
- `sql-lens`.
- `test-runner`.

The compose setup should support running subsets to keep local iteration fast.

## CI

Recommended GitHub Actions jobs:

- `format`: Rustfmt, Prettier.
- `lint`: Clippy, ESLint.
- `unit`: Rust and frontend unit tests.
- `protocol-fixtures`: protocol golden tests.
- `integration-mysql`: Docker MySQL compatibility tests.
- `frontend-e2e`: Playwright smoke tests.
- `security`: dependency audit and secret scan.
- `docs`: markdown lint and link check.

Heavy compatibility jobs for StarRocks, TiDB, and Doris can start as nightly or manual workflows.

## Quality Gates

Before merging protocol changes:

- Unit tests pass.
- Protocol fixtures pass.
- At least one live MySQL integration test passes.
- Redaction tests pass.

Before release:

- Full compatibility matrix passes or known gaps are documented.
- Benchmarks are run and compared with previous release.
- Web UI smoke tests pass.
