# SQL Lens Storage

## Overview

SQL Lens stores captured SQL events, connection state, and derived statistics.

The default storage backend is an in-memory ring buffer. Persistent storage is optional.

## Storage Goals

- Keep the capture path fast.
- Avoid blocking packet forwarding on slow storage.
- Use a protocol-neutral event model.
- Support efficient timeline queries.
- Support local persistence when configured.
- Avoid storing secrets after redaction.

## Core Entities

### SQL Event

Fields:

- `id`.
- `timestamp`.
- `protocol`.
- `database_type`.
- `connection_id`.
- `client_addr`.
- `backend_addr`.
- `user`.
- `database`.
- `kind`.
- `status`.
- `duration_ms`.
- `original_sql`.
- `normalized_sql`.
- `expanded_sql`.
- `fingerprint`.
- `parameters`.
- `rows`.
- `error`.
- `timings`.
- `metadata`.

### Connection

Fields:

- `id`.
- `protocol`.
- `database_type`.
- `client_addr`.
- `backend_addr`.
- `user`.
- `database`.
- `state`.
- `connected_at`.
- `closed_at`.
- `last_activity_at`.
- `bytes_in`.
- `bytes_out`.
- `query_count`.

### Prepared Statement

Fields:

- `connection_id`.
- `statement_key`.
- `protocol`.
- `template_sql`.
- `parameter_count`.
- `created_at`.
- `closed_at`.
- `metadata`.

## Ring Buffer

The ring buffer is the default backend.

Properties:

- In-memory.
- Fixed event capacity.
- Oldest events are evicted first.
- Fast append.
- Fast recent timeline reads.
- No persistence across restart.

Default:

```text
capacity = 100000 events
```

Recommended behavior:

- Maintain secondary in-memory indexes for common filters where cheap.
- Rebuild derived statistics from retained events.
- Record eviction counters.
- Return clear API metadata when results are truncated by retention.

## SQLite Storage

SQLite is the optional local persistence backend.

Use cases:

- Longer local debugging sessions.
- Sharing capture files.
- Post-run analysis.

Recommended tables:

- `sql_events`.
- `sql_parameters`.
- `connections`.
- `prepared_statements`.
- `schema_version`.

Recommended indexes:

- `sql_events(timestamp)`.
- `sql_events(protocol, timestamp)`.
- `sql_events(database_type, timestamp)`.
- `sql_events(database, timestamp)`.
- `sql_events(user, timestamp)`.
- `sql_events(status, timestamp)`.
- `sql_events(fingerprint, timestamp)`.
- `sql_events(duration_ms)`.

Current implementation:

- `sql-lens-storage` owns `SqliteEventStore` as a storage-local synchronous API.
- `SqliteEventStore::new` applies the schema before use.
- `insert_event` writes one redacted `SqlEvent` and its parameters in one SQLite transaction.
- Structured protocol metadata and parameter values are stored as JSON text.
- `query_timeline` reads persisted `sql_events` newest-first with storage-owned
  cursors and the shared SQL event filter contract.
- Runtime capture fan-out, file lifecycle configuration, retention cleanup, and
  API/runtime SQLite selection are separate tasks.

## DuckDB Future

DuckDB is planned for analytical workloads.

Use cases:

- Long capture analysis.
- Columnar aggregations.
- Local reports.
- Import/export workflows.

DuckDB should not replace the ring buffer hot path in early releases.

## Retention

Retention dimensions:

- Maximum event count.
- Maximum age.
- Maximum storage bytes.

Drop policy:

- Default: drop oldest.
- Optional: reject new capture events when full.

Retention must apply after redaction.

## Query Optimization

Timeline query:

- Sort by timestamp descending.
- Cursor-based pagination.
- Avoid offset pagination for large stores.

Search:

- Use normalized SQL and fingerprint indexes.
- For SQLite text search, consider FTS later.
- Avoid expensive wildcard scans on the hot path.

Statistics:

- Use incremental counters for live dashboard.
- Use storage queries for historical ranges.
- Cache common windows such as 1m, 5m, 15m.

## Immutability

Captured SQL events should be immutable after finalization.

Allowed updates:

- Connection state.
- Derived statistics.
- Redaction policy metadata for future migrations.

Not allowed:

- Mutating stored SQL event contents after display unless running an explicit migration.

## Privacy

Storage must never persist:

- Database passwords.
- Raw authentication payloads.
- TLS private keys.
- Unredacted sensitive parameters when redaction is enabled.
