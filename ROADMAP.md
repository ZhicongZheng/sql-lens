# SQL Lens Roadmap

## Roadmap Principles

- Build the smallest reliable proxy first.
- Keep protocol adapters independent from core capture models.
- Make local debugging excellent before building enterprise workflows.
- Prefer correctness and testability over broad database coverage.
- Avoid production-gateway promises until the proxy has years of hardening.

## v0.1: Proxy Foundation

Goal: establish the repository, runtime shape, and capture pipeline.

Scope:

- Rust workspace.
- CLI entry point.
- Configuration loading.
- TCP listener.
- Bidirectional forwarding.
- Connection lifecycle events.
- Shared capture event model.
- Ring buffer storage abstraction.
- Basic REST health endpoint.
- Basic WebSocket event stream.
- Logging and redaction foundations.
- Contributor documentation and CI design.

Exit criteria:

- SQL Lens can forward TCP bytes from a local port to a configured backend.
- Connections appear in a minimal API response.
- No protocol-specific SQL decoding is required yet.

## v0.5: MySQL-Compatible Protocol MVP

Goal: make the proxy protocol-aware for common MySQL-compatible traffic.

Scope:

- MySQL packet framing.
- Handshake forwarding.
- Authentication forwarding.
- Sequence ID tracking.
- COM_QUERY capture.
- COM_PING and COM_QUIT handling.
- COM_STMT_PREPARE tracking.
- COM_STMT_EXECUTE parameter decoding for common types.
- COM_STMT_CLOSE cleanup.
- Error packet parsing.
- Result summary capture.
- Protocol test fixtures.
- Docker Compose compatibility tests for MySQL and at least one compatible database.

Exit criteria:

- A common MySQL client can connect through SQL Lens.
- Text queries and prepared statement executions appear in captured events.
- Basic expanded SQL is visible through API and WebSocket.

## v1.0: First Usable Debugger

Goal: ship a useful local SQL debugging product.

**Status:** largely implemented in-tree for MySQL-compatible local use. Remaining gap is primarily packaging / release readiness and broader real-client hardening—not greenfield feature work.

Scope (done or substantially present):

- Web UI (dashboard, SQL list/detail, connections, statistics, settings, replay).
- Search filters, slow / error classification.
- Guarded replay API + UI safeguards.
- Configuration docs, redaction baseline.
- Single-process static UI delivery (`web.static_dir` / auto-discover).

Still open for a formal v1.0 release:

- Published binaries / installers / container image.
- Broader real-client boundary test matrix.
- Screenshot and install polish.

Exit criteria:

- A developer can install SQL Lens, point an application at it, and inspect captured SQL from a browser.
- Prepared statements are understandable for common workloads.
- Sensitive data is redacted when configured.

## v1.5: Persistence And Extensibility

Goal: make SQL Lens extensible and useful for longer debugging sessions.

**Status (partial):** SQLite storage, retention policies, SQL fingerprinting, SQL export, plugin *hook traits + isolated app runtime*, and a stable protocol adapter registry already exist. Remaining v1.5 themes are mostly **exporters and richer plugin/exporter productization**.

Scope:

- ~~SQLite storage backend~~ (implemented).
- ~~Retention policies~~ (age/count; max_bytes unsupported).
- ~~Plugin hook API / in-process runtime~~ (implemented; no remote install).
- Exporter interface productization (webhook / Prometheus / OpenTelemetry).
- ~~SQL fingerprinting~~ / ~~SQL export~~ (implemented).
- EXPLAIN helper.

Exit criteria:

- Users can persist captures locally.
- Plugin authors can observe query events without modifying proxy internals.
- Metrics can be exported to common observability systems.
## v2.0: Multi-Protocol Expansion

Goal: prove the architecture supports additional SQL execution surfaces.

Scope:

- PostgreSQL protocol adapter.
- PostgreSQL prepared statement lifecycle.
- ClickHouse feasibility and implementation path.
- SQLite tracing or driver integration feasibility.
- DuckDB analytics backend.
- Multi-protocol UI refinements.
- Protocol adapter compatibility test suite.

Exit criteria:

- At least one non-MySQL protocol family works through the shared capture model.
- The core API and UI do not require protocol-specific rewrites.

## Backlog Themes

- Protocol coverage.
- Storage and retention.
- Search and analytics.
- Replay safety.
- Security and redaction.
- UI quality.
- Contributor experience.
- Packaging and distribution.
- Observability integrations.

