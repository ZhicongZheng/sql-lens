# Issue 112 Design: Runtime SQLite Persistence Fan-Out

## Scope

Wire configured SQLite persistence into `sql-lens-app` runtime while keeping the current ring-buffer API state as the live serving surface.

## Runtime Shape

`sql-lens-app` will derive a runtime storage sink from `SqlLensConfig.storage`:

- `StorageType::RingBuffer`: no persistent sink.
- `StorageType::Sqlite`: open/migrate `storage.path` through `sql-lens-storage`, then create a worker channel.
- `StorageType::DuckDb`: currently unsupported for runtime startup and should return a clear runtime error if selected.

The SQLite sink is a small non-blocking fan-out:

```text
captured events
  -> classify
  -> WebSocket broadcast
  -> live statistics
  -> ring buffer append
  -> try_send SQLite worker channel
```

The worker owns the synchronous `SqliteEventStore` and runs blocking inserts outside the forwarding task. If the channel is full or closed, runtime logs a warning and continues forwarding.

## Storage Boundary

`sql-lens-storage` should expose a path-based constructor:

```rust
impl SqliteEventStore {
    pub fn open(path: impl AsRef<std::path::Path>) -> rusqlite::Result<Self>;
}
```

This keeps rusqlite connection setup inside the storage crate. `sql-lens-app` should not construct `rusqlite::Connection` directly.

## Error Behavior

- Empty SQLite path: `MinimalMysqlRuntimeError::StorageConfig`.
- SQLite open/migration error: `MinimalMysqlRuntimeError::SqliteStorage`.
- Worker join failure during shutdown: reuse `MinimalMysqlRuntimeError::Join`.
- Per-event insert failure after runtime startup: warning log only.

## Compatibility

Ring-buffer default behavior remains unchanged. REST endpoints continue reading from the existing `ApiState` ring buffer; SQLite-backed historical API selection is a later task.

