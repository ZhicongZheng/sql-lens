# Wire configured SQLite storage into app runtime

## Goal

Wire `storage.type = "sqlite"` configuration into `sql-lens-app` runtime so captured SQL events can be persisted to SQLite while the API continues serving live ring-buffer view, without blocking packet forwarding.

## Background

This task depends on:
- Issue 087: SQLite event storage implementation
- Issue 109: App runtime startup wiring

The storage configuration layer needs to initialize SQLite persistence when configured, while maintaining ring-buffer behavior for live queries.

## Requirements

- Runtime must detect `storage.type = "sqlite"` and `storage.path` configuration
- Runtime must fail startup with clear error when SQLite selected without valid path
- Captured events must append to ring buffer (for live statistics/API) and persist to SQLite asynchronously
- SQLite persistence failures must be logged as warnings without stopping proxy forwarding
- Default ring-buffer-only behavior must remain unchanged when `storage.type` is not set or is "ring_buffer"

## Acceptance Criteria

- [ ] Runtime initializes SQLite storage when `storage.type = "sqlite"` and `storage.path` is configured
- [ ] Runtime startup fails clearly when SQLite storage is selected without a usable path
- [ ] Captured SQL events continue to append to the ring buffer and update live statistics
- [ ] Captured SQL events are also persisted to SQLite without blocking packet forwarding
- [ ] SQLite persistence failures are logged as warnings and do not stop proxy forwarding
- [ ] Default ring-buffer runtime behavior is unchanged
- [ ] Tests cover ring-buffer-only startup and SQLite-configured persistence with a temporary database path

## Technical Notes

**Confirmed from codebase inspection:**
- `RuntimeStorage::from_config()` in `crates/sql-lens-app/src/lib.rs:381-418` handles `StorageType::Sqlite`:
  - Opens `SqliteEventStore` at configured path
  - Creates separate reader for API queries
  - Initializes `EventPersistence::sqlite(store)` with background worker thread
  - Returns error `MinimalMysqlRuntimeError::SqliteStorage` on open failure
- `sqlite_storage_path()` at line 444 validates path requirement with clear error message
- `EventPersistence::sqlite()` at line 461 spawns worker thread for async inserts
- Tests exist: `store_sql_events_persists_to_sqlite_worker` (line 1550), `sqlite_worker_insert_failure_does_not_stop_capture_state` (line 1585)
- DuckDB returns explicit "not supported yet" error (line 415-417)

**Scope clarification:**
- Issue 112 confirms SQLite storage wiring in app runtime is complete and tested
- All AC requirements are satisfied by existing implementation in `RuntimeStorage::from_config()` and `EventPersistence::sqlite()`
- No production code changes expected
- Task complexity: Medium (6h) — verification, documentation, test validation

**Dependencies:**
- Issue 087: SQLite event storage (provides `SqliteEventStore`, `EventPersistence::sqlite`)
- Issue 109: App runtime startup (provides `start_minimal_mysql_runtime_with_runtime_storage`)
