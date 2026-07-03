# SQL Lens Benchmark Plan

## Performance Goals

Initial targets for local development and staging workloads:

- Capture path: 10,000+ QPS.
- Proxy overhead: less than 1 ms p95 in normal local usage.
- Memory: less than 100 MB with default ring buffer configuration.
- CPU: low enough to run beside a developer application.
- Connections: 500 active connections for v1 target.
- Event loss: zero under target load; explicit counters when overload policy drops events.

These are targets, not guarantees. Each release should publish measured results.

## Benchmark Dimensions

### Latency

Measure:

- Direct client to database latency.
- Client to SQL Lens to database latency.
- Added proxy overhead.
- p50, p95, p99.

### Throughput

Measure:

- Queries per second.
- Prepared statement executions per second.
- WebSocket subscribers impact.
- Storage backend impact.

### Memory

Measure:

- Baseline idle memory.
- Memory per connection.
- Memory per retained event.
- Ring buffer capacity behavior.
- Large SQL and binary parameter handling.

### CPU

Measure:

- Packet forwarding CPU.
- Protocol parsing CPU.
- SQL expansion CPU.
- Redaction CPU.
- WebSocket broadcast CPU.

### Connections

Measure:

- Active idle connections.
- Active querying connections.
- Connect/disconnect churn.
- Backend dial latency.

## Tools

Recommended tools:

- `sysbench` for MySQL workloads.
- `mysqlslap` for simple MySQL load.
- Custom driver benchmark for prepared statements.
- `wrk` or `oha` for REST API load.
- Tokio metrics for runtime observation.
- Criterion for Rust microbenchmarks.

## Scenarios

### Baseline Forwarding

SQL Lens forwards traffic without protocol capture.

Purpose:

- Measure minimum proxy overhead.

### Text Query Capture

Capture `COM_QUERY` traffic.

Purpose:

- Measure packet parsing and event creation cost.

### Prepared Statement Capture

Capture prepare and execute traffic.

Purpose:

- Measure parameter decoding and SQL expansion cost.

### Redaction Enabled

Run with configured redaction rules.

Purpose:

- Measure privacy features impact.

### WebSocket Subscribers

Run with 1, 5, and 20 live UI subscribers.

Purpose:

- Measure live broadcast cost.

### SQLite Storage

Run with persistent storage enabled.

Purpose:

- Measure write overhead and query latency.

## Reporting Template

Each benchmark report should include:

- SQL Lens version.
- Commit SHA.
- OS.
- CPU.
- Memory.
- Database version.
- Driver.
- Config file.
- Workload command.
- Direct database result.
- SQL Lens result.
- Overhead.
- Notes.

## Regression Policy

A release should investigate:

- More than 10 percent p95 latency regression.
- More than 10 percent QPS regression.
- More than 15 percent memory regression.
- Any unbounded memory growth.
- Any capture event loss under target load.

