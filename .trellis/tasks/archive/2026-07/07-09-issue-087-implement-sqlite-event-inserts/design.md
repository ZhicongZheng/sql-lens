# Design

## Boundary

Implement SQLite insert/readback support inside `sql-lens-storage`. This keeps persistence logic in the storage crate and avoids wiring SQLite into app/runtime capture paths in this task.

## Public Contract

Add a small SQLite store wrapper around `rusqlite::Connection`:

```rust
pub struct SqliteEventStore { ... }

impl SqliteEventStore {
    pub fn new(connection: rusqlite::Connection) -> rusqlite::Result<Self>;
    pub fn insert_event(&mut self, event: &SqlEvent) -> rusqlite::Result<()>;
    pub fn get_event_row(&self, id: &SqlEventId) -> rusqlite::Result<Option<SqliteEventRow>>;
}
```

The readback row is test/support-oriented and should expose enough stored fields to prove insert correctness. Issue 088 can add full timeline/domain reconstruction.

## Redaction

`insert_event` should call `redact_sql_event(event.clone(), &RedactionPolicy::default())` before writing. This mirrors `RingBufferStore::append` and ensures SQLite never stores sensitive parameters when default redaction is enabled.

## Storage Shape

- `sql_events` stores scalar event fields, result summary, error summary, timings, and metadata JSON.
- `sql_parameters` stores parameter index, name, value type, JSON value, and redaction flag.
- Inserts should be transactional so event and parameters stay consistent.

## Compatibility

This task does not alter API, proxy, app startup, or ring buffer behavior.
