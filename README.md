# SQL Lens

> See the SQL your application actually executes.

SQL Lens is a developer-first SQL Debug Proxy. It sits between an application and a database, captures the SQL traffic that actually crosses the wire, reconstructs prepared statements, and streams the result into a local web UI.

It is inspired by Charles, Fiddler, and mitmproxy, but for SQL database protocols.

```text
Application
    |
    v
SQL Lens
    |
    v
Database
```

SQL Lens is not a database middleware, query router, sharding layer, high availability proxy, or governance platform. Its job is debugging, observability, audit, and analysis for development and pre-production workflows.

## Why

Application logs often show incomplete SQL. ORMs hide generated queries. Prepared statements split SQL templates from parameters. Database logs may be unavailable, noisy, delayed, or too invasive for local debugging.

SQL Lens gives developers a direct view of:

- The SQL an application actually executes.
- The final values bound to prepared statements.
- Slow and failed queries.
- Connection behavior.
- Query timing and result summaries.
- Searchable SQL history.
- Replay-ready SQL details.

The main integration requirement is changing the database address from the real database to SQL Lens.

## Protocol Strategy

SQL Lens is designed as a multi-protocol SQL debug proxy.

The first production target is the MySQL-compatible protocol family:

- MySQL
- StarRocks
- TiDB
- Apache Doris
- Other MySQL protocol compatible databases

Future protocol targets include:

- PostgreSQL
- SQLite integration, subject to protocol and driver feasibility
- ClickHouse native protocol or HTTP SQL interface

Protocol-specific parsing must stay behind protocol adapters. The API, storage model, UI, plugin system, and event pipeline use shared capture models with extensible metadata.

## Features

Planned v1 features:

- Transparent TCP proxy.
- MySQL-compatible handshake and authentication forwarding.
- SQL capture for text queries.
- Prepared statement lifecycle tracking.
- Prepared statement parameter expansion.
- Query timeline.
- Slow SQL detection.
- Error SQL detection.
- Connection list.
- SQL statistics.
- REST API.
- WebSocket live updates.
- Local dashboard.
- Ring buffer storage by default.

Future features:

- SQL replay.
- SQL export.
- SQL fingerprinting.
- EXPLAIN helper.
- SQLite persistence.
- DuckDB analytics.
- PostgreSQL support.
- Plugin hooks.
- Prometheus and OpenTelemetry exporters.

## Screenshot

Screenshots are placeholders until the web UI exists.

```text
+-------------------------------------------------------------+
| SQL Lens                                                    |
| Dashboard | SQL | Connections | Statistics | Settings       |
+-------------------------------------------------------------+
| QPS 120 | p95 8.4 ms | Slow SQL 4 | Errors 1 | Conn 18     |
+-------------------------------------------------------------+
| Time        | DB      | User | Latency | Status | SQL       |
| 12:00:01.1  | app     | root | 3.2 ms  | OK     | SELECT... |
| 12:00:02.4  | billing | svc  | 180 ms  | SLOW   | UPDATE... |
+-------------------------------------------------------------+
| SQL detail: original SQL, expanded SQL, params, timings      |
+-------------------------------------------------------------+
```

## Installation

SQL Lens is not implemented yet. The intended installation channels are:

- Prebuilt binaries from GitHub Releases.
- Homebrew tap.
- Docker image.
- Cargo install for source users.

Target commands:

```bash
brew install sql-lens/tap/sql-lens
docker run --rm -p 3307:3307 -p 5173:5173 sql-lens/sql-lens
cargo install sql-lens
```

## Quick Start

Target v1 workflow:

1. Start SQL Lens:

   ```bash
   sql-lens --config sql-lens.toml
   ```

2. Configure SQL Lens:

   ```toml
   [proxy]
   listen = "127.0.0.1:3307"

   [backend]
   address = "127.0.0.1:3306"
   protocol = "mysql"

   [web]
   listen = "127.0.0.1:5173"
   ```

3. Change the application database address:

   ```text
   mysql://user:password@127.0.0.1:3307/app
   ```

4. Open the dashboard:

   ```text
   http://127.0.0.1:5173
   ```

## Architecture

SQL Lens is split into small, testable modules:

```text
client connection
      |
      v
proxy listener
      |
      v
protocol adapter
      |
      +-- packet forwarding
      +-- command decoding
      +-- prepared statement tracking
      |
      v
capture pipeline
      |
      +-- ring buffer storage
      +-- optional persistent storage
      +-- websocket broadcast
      |
      v
REST API and Web UI
```

Recommended Rust workspace:

```text
crates/
  sql-lens-core/
  sql-lens-proxy/
  sql-lens-protocol/
  sql-lens-protocol-mysql/
  sql-lens-storage/
  sql-lens-api/
  sql-lens-plugin/
  sql-lens-app/
web/
docs/
examples/
tests/
```

See [ARCHITECTURE.md](ARCHITECTURE.md) for the full design.

## Roadmap

- v0.1: TCP proxy foundation and capture event model.
- v0.5: MySQL-compatible protocol parsing and prepared statements.
- v1.0: Usable local SQL debugging proxy with web UI.
- v1.5: Plugin system, exporters, SQLite persistence, stable extension points.
- v2.0: Additional protocol families such as PostgreSQL and ClickHouse.

See [ROADMAP.md](ROADMAP.md) and [MILESTONE.md](MILESTONE.md).

## Project Recommendations

### Directory Structure

Use a Rust workspace plus a separate web application:

```text
sql-lens/
  crates/
  web/
  docs/
  examples/
  tests/
```

### Rust Crate Split

- `sql-lens-core`: protocol-neutral domain models.
- `sql-lens-proxy`: TCP proxy, sessions, forwarding, shutdown.
- `sql-lens-protocol`: protocol adapter traits and registry.
- `sql-lens-protocol-mysql`: MySQL-compatible protocol adapter.
- `sql-lens-storage`: ring buffer, SQLite, future DuckDB.
- `sql-lens-api`: REST and WebSocket API.
- `sql-lens-plugin`: hooks and exporters.
- `sql-lens-app`: CLI and service composition.

### Optional Go Package Split

Rust is the recommended backend. If a Go implementation or sidecar is explored later, use:

```text
cmd/sql-lens/
internal/core/
internal/proxy/
internal/protocol/
internal/protocol/mysql/
internal/storage/
internal/api/
internal/plugin/
pkg/sqlcapture/
```

Keep the same rule: protocol adapters depend on shared capture models, not the reverse.

### React Structure

Use the structure documented in [UI.md](UI.md):

```text
web/src/app
web/src/components
web/src/features
web/src/lib
web/src/types
web/src/styles
```

### GitHub Labels

Recommended labels:

- `type:feature`
- `type:bug`
- `type:task`
- `type:test`
- `type:docs`
- `type:research`
- `area:proxy`
- `area:protocol-mysql`
- `area:protocol-postgresql`
- `area:storage`
- `area:api`
- `area:frontend`
- `area:security`
- `area:plugin`
- `area:ci`
- `priority:P0`
- `priority:P1`
- `priority:P2`
- `difficulty:easy`
- `difficulty:medium`
- `difficulty:hard`
- `good-first-issue`
- `help-wanted`

### GitHub Project Board

Recommended columns:

- Inbox.
- Ready.
- In Progress.
- Review.
- Testing.
- Blocked.
- Done.

Recommended fields:

- Milestone.
- Priority.
- Area.
- Difficulty.
- Estimated time.
- Protocol family.
- Release target.

### CI/CD

Recommended GitHub Actions:

- Rust format, lint, and tests.
- Frontend typecheck, lint, tests, and build.
- Protocol fixture tests.
- MySQL integration tests.
- Markdown lint and link check.
- Security audit.
- Release packaging workflow.

### Release Strategy

- Publish alpha releases early for contributors.
- Use release candidates before v1.0.
- Publish binaries, checksums, Docker images, and release notes.
- Keep compatibility notes explicit for each database.

### SemVer

Use SemVer:

- Patch: fixes and documentation.
- Minor: backward-compatible features.
- Major: breaking API, config, plugin, or storage format changes.

Before v1.0, breaking changes are allowed but must be called out in release notes.

### License

Recommended license: Apache License 2.0.

Rationale:

- Permissive for open source and commercial users.
- Patent protection.
- Friendly to future enterprise offerings.

### Logo Style

Recommended visual direction:

- A lens or magnifier shape.
- SQL cursor or query line detail.
- Subtle database cylinder reference.
- Works in monochrome.
- Avoid vendor-specific database symbols.
- Strong dark-mode version.

### Website Homepage

Homepage should be product-led, not a generic marketing splash:

- First viewport: SQL Lens name, slogan, and a real product screenshot once available.
- Immediate quick start.
- Architecture diagram.
- Prepared statement expansion example.
- Supported databases.
- Roadmap.
- Contributing callout.

### Future Commercialization

Keep the open source edition strong:

- Local proxy.
- SQL capture.
- Prepared statement expansion.
- Dashboard.
- Search.
- Ring buffer storage.
- Extensible protocol architecture.

Potential enterprise additions:

- Team workspaces.
- Centralized retention.
- SSO and RBAC.
- Audit reports.
- Shared capture environments.
- Compliance exports.
- Managed cloud dashboard.

## FAQ

### Is SQL Lens a production database proxy?

No. It is designed for development, local debugging, CI, and pre-production observability. Production use may become possible later, but it is not the primary product promise.

### Does SQL Lens modify SQL?

No by default. SQL Lens observes and forwards traffic. SQL rewrite is a non-goal for the open source core.

### Does SQL Lens store passwords?

It should never persist database passwords. Authentication packets may pass through the proxy, but secrets must not be written to logs, APIs, or storage.

### Why start with MySQL-compatible protocols?

MySQL, StarRocks, TiDB, and Doris share enough protocol behavior to make the first version useful without building multiple protocol stacks at once.

### Will PostgreSQL be supported?

The architecture must allow it. PostgreSQL is planned after the MySQL-compatible foundation is stable.

### What about SQLite?

SQLite is embedded and does not expose the same client/server protocol model by default. Support requires a separate tracing or driver integration design.

## Contributing

Contributions should stay small, testable, and aligned with the roadmap. Start with [CONTRIBUTING.md](CONTRIBUTING.md), [AGENTS.md](AGENTS.md), and the issue backlog in [ISSUES.md](ISSUES.md).

## License

Recommended license: Apache License 2.0.

Apache 2.0 is friendly to open source users, commercial users, and future hosted or enterprise offerings while preserving patent protection.
