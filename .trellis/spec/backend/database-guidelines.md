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

## Scenario: SQLite Event Inserts

### 1. Scope / Trigger

- Trigger: a task persists `SqlEvent` rows into SQLite or changes the SQLite
  persistence API.
- SQLite persistence is storage-local until a separate runtime wiring task.
- Inserts must not put SQLite calls in protocol, proxy, API handlers, or app
  startup code.

### 2. Signatures

Current storage-local API:

```rust
pub struct SqliteEventStore;

impl SqliteEventStore {
    pub fn new(connection: rusqlite::Connection) -> rusqlite::Result<Self>;
    pub fn insert_event(&mut self, event: &SqlEvent) -> rusqlite::Result<()>;
    pub fn get_event_row(&self, id: &SqlEventId) -> rusqlite::Result<Option<SqliteEventRow>>;
    pub fn get_parameter_rows(&self, id: &SqlEventId) -> rusqlite::Result<Vec<SqliteParameterRow>>;
}
```

### 3. Contracts

- `new` applies `apply_sqlite_schema` before returning a store.
- `insert_event` applies `redact_sql_event(event.clone(), &RedactionPolicy::default())`
  before writing.
- One `SqlEvent` row is inserted into `sql_events`; its parameters are inserted
  into `sql_parameters`.
- Event and parameter inserts are written in one SQLite transaction.
- Structured protocol metadata and parameter values are serialized as JSON text.
- Duplicate event IDs are rejected by the `sql_events.id` primary key; do not
  silently replace existing events unless a future task adds update semantics.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| Schema is missing | `SqliteEventStore::new` applies it |
| JSON serialization fails | Return a `rusqlite::Error` without partial writes |
| Event row insert fails | Transaction rolls back; no parameter rows remain |
| Parameter row insert fails | Transaction rolls back; no event row remains |
| Duplicate event ID | Return the SQLite constraint error |
| Missing event readback | Return `Ok(None)` |

### 5. Good/Base/Bad Cases

Good:

- A caller opens a `rusqlite::Connection`, constructs `SqliteEventStore`, and
  inserts captured events through `insert_event`.
- Tests verify stored SQL and parameters are redacted.

Base:

- `get_event_row` and `get_parameter_rows` are test/support-oriented readback
  helpers until the timeline query task adds a query API.

Bad:

- Writing unredacted `SqlEvent` parameters to SQLite.
- Using `INSERT OR REPLACE` without a documented event update contract.
- Calling SQLite directly from protocol observers or TCP forwarding code.

### 6. Tests Required

- In-memory SQLite store initialization applies the schema.
- Insert/readback covers scalar `sql_events` columns.
- Parameter row insertion covers index, name, value type, JSON value, and
  redaction flag.
- Redaction tests assert sensitive SQL text and parameter values are masked
  before persistence.
- Duplicate ID behavior is asserted.
- Run `cargo fmt --check`, `cargo test --workspace`, and
  `cargo clippy --workspace --all-targets -- -D warnings`.

### 7. Wrong vs Correct

#### Wrong

```rust
connection.execute("INSERT INTO sql_events ...", params![event.original_sql])?;
```

#### Correct

```rust
let event = redact_sql_event(event.clone(), &RedactionPolicy::default());
let tx = connection.transaction()?;
// Insert sql_events and sql_parameters in tx, then commit.
```
