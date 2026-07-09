# SQL Lens Product Requirements

## Product Positioning

SQL Lens is a developer-first SQL Debug Proxy.

It helps engineers see the SQL their applications actually execute by transparently proxying database traffic, reconstructing SQL events, and presenting them in a local web UI.

SQL Lens is not a database governance platform, production traffic router, high availability layer, sharding system, SQL rewrite engine, or data synchronization tool.

## Slogan

See the SQL your application actually executes.

## Target Users

- Backend engineers debugging ORM-generated SQL.
- Full-stack engineers investigating local and staging database behavior.
- Database engineers reviewing slow and failed queries.
- SRE and platform engineers adding lightweight SQL observability to non-production environments.
- Open source contributors building protocol adapters, storage engines, exporters, and UI features.
- AI coding agents that need stable boundaries and clear module contracts.

## Problems

- Prepared statements hide parameter values from normal SQL logs.
- ORMs and query builders make it hard to find generated SQL.
- Database logs are often disabled, inaccessible, or too noisy.
- Local debugging tools are strong for HTTP but weak for database protocols.
- Teams need a searchable timeline of SQL activity without changing application code.

## Goals

- Transparent integration by changing database address.
- Capture text queries and prepared statement executions.
- Expand prepared statement parameters into readable SQL.
- Show live query timeline.
- Surface slow SQL and error SQL.
- Provide connection visibility.
- Provide search and statistics.
- Support SQL replay workflows.
- Keep the first implementation small and stable.
- Preserve extension points for multiple database protocol families.

## Non-Goals

- Production database gateway replacement.
- Query optimizer.
- SQL rewrite engine.
- Database firewall.
- Sharding.
- Read/write splitting.
- High availability failover.
- Data replication.
- Policy governance platform.
- Full database audit compliance product in the open source MVP.

## Protocol Scope

### v1 Protocol Family

The first version supports the MySQL-compatible protocol family:

- MySQL
- StarRocks
- TiDB
- Apache Doris

### Future Protocol Families

- PostgreSQL protocol.
- SQLite tracing or driver-based integration, if feasible.
- ClickHouse native protocol or HTTP SQL interface.

### Product Requirement

The product model must not assume every SQL event came from MySQL. Shared fields should describe common SQL activity, and protocol-specific data should live in `metadata`.

## Core Features

### Transparent Proxy

Users configure SQL Lens as the database endpoint. SQL Lens forwards traffic to the real database and captures protocol events.

Acceptance:

- Application code does not need to change.
- Database driver connection strings only change host and port.
- SQL Lens forwards unsupported packets safely where possible.

### SQL Capture

Capture each SQL execution with timing, connection, status, and result summary.

Required fields:

- Event ID.
- Timestamp.
- Protocol.
- Database type.
- Connection ID.
- Client address.
- Backend address.
- User.
- Database.
- SQL kind.
- Original SQL.
- Normalized SQL.
- Expanded SQL when available.
- Duration.
- Status.
- Error summary.
- Row summary when available.
- Metadata.

### Prepared Statement Expansion

SQL Lens reconstructs prepared statement executions from prepare and execute protocol messages.

Acceptance:

- Track statement ID to SQL template per connection.
- Decode bound parameters.
- Render expanded SQL for display.
- Preserve original SQL and raw parameter list.
- Redact sensitive values when configured.
- Mark unknown or unsupported values explicitly.

### SQL Timeline

The timeline is the primary debugging view.

Acceptance:

- Show newest SQL events live.
- Filter by protocol, database, user, status, latency, and text.
- Keep list interactions fast with ring buffer storage.

### Slow SQL

Slow SQL is any event whose duration exceeds configured thresholds.

Acceptance:

- Global threshold.
- Optional per-database threshold.
- Slow status in SQL list.
- Slow count and latency charts in Dashboard.

### Error SQL

Error SQL captures failed command responses.

Acceptance:

- Store database error code when available.
- Store SQL state when available.
- Store sanitized error message.
- Display error details in SQL Detail.

### SQL Replay

Replay helps developers rerun captured SQL against a selected target.

MVP requirement:

- Design the UI and API surface.
- Do not execute replay automatically without explicit user action.
- Warn when replaying mutating SQL.

### Search

Search supports quick investigation.

Filters:

- SQL text.
- Fingerprint.
- Protocol.
- Database type.
- Database.
- User.
- Client IP.
- Status.
- Duration range.
- Time range.

### Statistics

Statistics summarize traffic.

Metrics:

- QPS.
- Error rate.
- Slow query count.
- Latency percentiles.
- Top fingerprints.
- Top databases.
- Top users.
- Active connections.

## Product Areas

### Dashboard

Purpose: show the current health and shape of SQL traffic.

Widgets:

- QPS.
- p50, p95, p99 latency.
- Active connections.
- Slow SQL count.
- Error SQL count.
- Protocol mix.
- Top slow fingerprints.
- Top error fingerprints.

### SQL List

Purpose: scan and filter captured SQL.

Columns:

- Time.
- Protocol.
- Database.
- User.
- Client.
- Duration.
- Status.
- Rows.
- SQL preview.

### SQL Detail

Purpose: inspect one SQL event.

Sections:

- Summary.
- Original SQL.
- Expanded SQL.
- Parameters.
- Timing.
- Connection.
- Result.
- Error.
- Protocol metadata.
- Replay controls.

### Connections

Purpose: inspect active and recent connections.

Fields:

- Connection ID.
- Protocol.
- Client address.
- Backend address.
- User.
- Database.
- State.
- Connected at.
- Last activity.
- Bytes in.
- Bytes out.
- Query count.

### Statistics

Purpose: aggregate query behavior.

Views:

- Time series.
- Fingerprints.
- Databases.
- Users.
- Errors.
- Latency distributions.

### Settings

Purpose: configure local behavior.

Areas:

- Proxy target.
- Storage.
- Retention.
- Redaction.
- Slow SQL thresholds.
- Exporters.

## Release Scope

### v0.1

Foundation release for contributors:

- Workspace structure.
- TCP proxy skeleton.
- Capture event model.
- Ring buffer interface.
- API shape.

### v0.5

First protocol-aware release:

- MySQL-compatible handshake forwarding.
- COM_QUERY capture.
- Prepared statement tracking.
- Basic REST and WebSocket.

### v1.0

Usable local debugger:

- Web UI.
- SQL list and detail.
- Dashboard.
- Search.
- Slow SQL.
- Error SQL.
- Basic replay preparation.

### v1.5

Extension release:

- SQLite storage.
- Plugin API.
- Prometheus exporter.
- OpenTelemetry exporter.
- Stable protocol adapter API.

### v2.0

Multi-protocol expansion:

- PostgreSQL support.
- ClickHouse support research or implementation.
- SQLite integration research or implementation.
- DuckDB analytics.

## Success Metrics

- A developer can inspect real SQL within 5 minutes of installation.
- Prepared statement expansion is correct for common MySQL drivers.
- Proxy overhead is below 1 ms p95 under local development workloads.
- The default memory footprint stays below 100 MB for normal local usage.
- New contributors can complete a scoped issue without asking where code belongs.

## Future Commercialization

The open source edition should remain useful for local and team development.

Possible paid offerings:

- Team collaboration.
- Shared environments.
- Long-term retention.
- Centralized audit reports.
- Advanced compliance exports.
- Managed cloud dashboard.

Commercial features must not remove the core open source value: transparent SQL capture, prepared statement expansion, local dashboard, and extensible protocol architecture.
