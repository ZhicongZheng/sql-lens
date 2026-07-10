# SQLite Storage Wiring Design

## Architecture

### Components

1. **RuntimeStorage Factory** (existing)
   - `RuntimeStorage::from_config()` parses `StorageConfig`
   - Creates `RingBufferStore` for live queries (always present)
   - Conditionally creates SQLite persistence based on `storage_type`

2. **EventPersistence** (existing)
   - `EventPersistence::default()` — ring-buffer only, no-op persistence
   - `EventPersistence::sqlite(store)` — spawns background worker thread for async inserts

3. **Storage Configuration** (existing)
   - `StorageConfig` with `storage_type` (RingBuffer/Sqlite/DuckDb) and `path`
   - `StorageType` enum in `sql-lens-config`

### Data Flow

```
StorageConfig → RuntimeStorage::from_config()
                    ↓
            RingBufferStore (always) + SqliteEventStore (if configured)
                    ↓
            EventPersistence (default or sqlite worker)
                    ↓
            store_sql_events() → ring buffer + async SQLite insert
```

### Boundaries

- **Issue 112 scope**: Confirm `StorageType::Sqlite` path works end-to-end
- **Issue 113 scope**: SQLite-backed API reads (`GET /api/v1/sql-events`)
- **Future scope**: DuckDB support, per-storage retention policies

## Contracts

### RuntimeStorage Initialization

```rust
impl RuntimeStorage {
    fn from_config(config: &StorageConfig) -> Result<Self, MinimalMysqlRuntimeError> {
        // Always creates ring buffer for live queries
        let event_store = RingBufferStore::new(...);

        match config.storage_type {
            StorageType::RingBuffer => { /* sqlite_event_reader: None, persistence: default */ }
            StorageType::Sqlite => {
                let path = sqlite_storage_path(config)?; // validates path required
                let store = SqliteEventStore::open(&path)?;
                let sqlite_event_reader = SqliteEventStore::open(&path)?;
                let (persistence, sqlite_worker) = EventPersistence::sqlite(store);
                // Returns both ring buffer + SQLite persistence
            }
            StorageType::DuckDb => Err(MinimalMysqlRuntimeError::StorageConfig(
                "storage.type = \"duckdb\" is not supported by app runtime yet"
            )),
        }
    }
}
```

### Error Handling

- `MinimalMysqlRuntimeError::SqliteStorage { path, source }` — wraps rusqlite errors with path context
- `MinimalMysqlRuntimeError::StorageConfig(String)` — path validation and unsupported type errors
- SQLite worker failures logged as warnings, do not propagate to capture state

## Trade-offs

| Decision | Rationale |
|----------|-----------|
| Dual storage (ring buffer + SQLite) | Ring buffer provides O(1) live queries; SQLite provides durable persistence |
| Async SQLite worker thread | Non-blocking capture path; worker failures isolated from proxy forwarding |
| Separate reader connection | Avoids lock contention between capture writes and API reads |

## Compatibility & Migration

- Ring-buffer-only default unchanged — existing deployments unaffected
- SQLite configuration is additive — no breaking changes
- `storage.type = "duckdb"` explicitly rejected with clear error message

## Operational Considerations

- SQLite worker thread shutdown via `drop(persistence)` + `worker.join()`
- Temporary database paths used in tests (cleaned up after each test)
- Path validation fails fast at startup, not during first insert

