# Design

## Boundary

Add SQLite schema/migration support inside `sql-lens-storage`. This crate owns storage backends, and the initial schema is a storage concern, not an API or app runtime concern.

## Dependency Choice

Use `rusqlite` for the first schema/migration implementation. The migration operation is synchronous and local, and this task does not wire SQLite into async capture paths. A later insert/query task can decide whether to wrap calls in blocking tasks or introduce a different async boundary.

## Public Contract

Expose a minimal schema API:

```rust
pub const SQLITE_SCHEMA_VERSION: i64 = 1;

pub fn apply_sqlite_schema(connection: &rusqlite::Connection) -> Result<(), rusqlite::Error>;
```

The function creates tables and indexes if they do not exist, then records the schema version.

## Schema Shape

Tables:

- `schema_version(version integer primary key, applied_at text not null)`
- `sql_events`: one row per finalized SQL event, with scalar columns for common filters and text JSON columns for structured payloads.
- `sql_parameters`: one row per event parameter.
- `connections`: latest known connection lifecycle fields.
- `prepared_statements`: prepared statement metadata.

Indexes follow `STORAGE.md` recommendations for event timeline and filters.

## Compatibility

This task only adds a schema module. It must not change ring buffer behavior, API behavior, proxy behavior, or runtime startup.

## Validation

Use in-memory SQLite tests to apply the schema and assert expected tables, indexes, and version row exist.
