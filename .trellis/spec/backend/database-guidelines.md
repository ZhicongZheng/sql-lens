# Database Guidelines

> Storage and database conventions for SQL Lens backend code.

## Overview

SQL Lens currently has no external database, ORM, or migration system. The
implemented storage layer is in-memory and lives in `crates/sql-lens-storage`.
Future SQLite and DuckDB work should be introduced through explicit storage
tasks rather than leaking persistence choices into protocol, proxy, API, or app
crates.

The current storage examples are:

- `crates/sql-lens-storage/src/ring_buffer.rs` for SQL event timeline storage.
- `crates/sql-lens-storage/src/connection_store.rs` for recent connection state.
- `crates/sql-lens-storage/src/live_statistics.rs` for derived counters.

## Storage Ownership

- `sql-lens-storage` owns storage data structures, retention behavior, filters,
  pagination cursors, and storage-specific errors.
- Protocol crates emit already-structured events and must not write storage
  directly.
- API handlers translate query parameters into storage queries; they should not
  duplicate filtering logic that belongs to storage.
- Storage receives redacted events by default. `RingBufferStore::append` applies
  `redact_sql_event` before keeping the event.

## Query Patterns

- Use typed query structs such as `RingBufferTimelineQuery` instead of passing
  many loose parameters.
- Validate filters before scanning stored events. For example,
  `SqlEventFilter::validate` rejects invalid duration and timestamp ranges.
- Use cursor-based pagination for timelines. Current cursors are storage-owned
  sequence positions that the API encodes as opaque strings.
- Return cloned snapshots or pages when crossing out of storage; do not expose
  mutable internal queues.

## Capacity And Retention

- In-memory stores are bounded with `NonZeroUsize` capacities.
- When full, stores evict oldest entries unless an upsert replaces an existing
  connection.
- Append/upsert methods return outcome structs that identify stored, replaced,
  or evicted IDs.
- Capacity, length, and empty-state helpers should stay cheap and deterministic
  so API and tests can inspect store state without side effects.

## Migrations

- SQLite schema migration support starts in `sql-lens-storage`.
- The initial public contract is:

```rust
pub const SQLITE_SCHEMA_VERSION: i64 = 1;

pub fn apply_sqlite_schema(
    connection: &rusqlite::Connection,
) -> Result<(), rusqlite::Error>;
```

- Use `rusqlite::Connection::execute_batch` for schema-only migrations.
- Keep migrations idempotent with `CREATE TABLE IF NOT EXISTS`,
  `CREATE INDEX IF NOT EXISTS`, and `INSERT OR IGNORE` for schema version rows.
- Keep SQLite access in `sql-lens-storage`; do not call SQLite from protocol,
  proxy, API handler, or app startup code until an explicit runtime wiring task.
- Rollback/downgrade behavior is not implemented yet. Future schema versions
  must document upgrade and compatibility behavior before adding version > 1.

Required first-schema tables:

- `schema_version`
- `sql_events`
- `sql_parameters`
- `connections`
- `prepared_statements`

Required first-schema SQL event indexes:

- `timestamp`
- `(protocol, timestamp)`
- `(database_type, timestamp)`
- `(database_name, timestamp)`
- `(user_name, timestamp)`
- `(status, timestamp)`
- `(fingerprint, timestamp)`
- `duration_ms`

## Naming Conventions

- Rust storage types use domain names: `RingBufferStore`, `ConnectionStore`,
  `SqlEventFilter`, `RingBufferTimelinePage`.
- Outcome structs use the operation name plus `Outcome`, such as
  `RingBufferAppendOutcome` and `ConnectionUpsertOutcome`.
- Cursor types include the owning query surface in the name.
- JSON-facing names are owned by API DTOs, not storage structs.

## Tests Required

For storage changes:

- Capacity and eviction behavior.
- Upsert or append outcome fields.
- Filter validation and matching behavior.
- Pagination order and next-cursor behavior.
- Redaction before storage when events may contain sensitive SQL or parameters.
- SQLite schema tests using `Connection::open_in_memory` when schema/migration
  code changes.
- SQLite schema tests must assert required tables, indexes, version row, and
  idempotent migration behavior.
- `cargo fmt --check`.
- `cargo test --workspace`.
- `cargo clippy --workspace --all-targets -- -D warnings`.

## Common Mistakes

- Do not add database access to protocol, proxy, API handler, or app startup code
  just to make a test pass.
- Do not persist unredacted SQL parameters when redaction is enabled.
- Do not duplicate filter matching in API or UI layers.
- Do not introduce SQLite, DuckDB, or migrations as placeholder scaffolding.
