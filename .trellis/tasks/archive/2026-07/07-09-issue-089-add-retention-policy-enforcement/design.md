# Design

## Boundary

Implement retention primitives inside `sql-lens-storage` only. Do not wire
retention into app startup, config validation, API handlers, proxy forwarding,
background tasks, or frontend code in this issue.

## Ring Buffer Contract

Add a max-events cleanup method to `RingBufferStore`:

```rust
pub struct RingBufferRetentionOutcome {
    pub deleted_event_ids: Vec<SqlEventId>,
}

impl RingBufferStore {
    pub fn enforce_max_events(&mut self, max_events: NonZeroUsize) -> RingBufferRetentionOutcome;
}
```

Behavior:

- If `len <= max_events`, return an empty outcome.
- If `len > max_events`, evict oldest entries until `len == max_events`.
- Increment the same total-evicted counter used by capacity eviction.
- Return deleted IDs oldest-first.
- Do not change fixed capacity. This method applies a retention policy window to
  the currently retained data.

## SQLite Contract

Add SQLite retention outcome and cleanup methods:

```rust
pub struct SqliteRetentionOutcome {
    pub deleted_event_ids: Vec<SqlEventId>,
    pub deleted_event_count: usize,
    pub deleted_parameter_count: usize,
}

impl SqliteEventStore {
    pub fn delete_events_older_than(
        &mut self,
        cutoff: &Timestamp,
    ) -> rusqlite::Result<SqliteRetentionOutcome>;

    pub fn enforce_max_events(
        &mut self,
        max_events: NonZeroUsize,
    ) -> rusqlite::Result<SqliteRetentionOutcome>;
}
```

Behavior:

- Age cleanup deletes rows with `timestamp < cutoff`.
- Count cleanup keeps the newest `max_events` rows using the same deterministic
  ordering as timeline queries: `timestamp DESC, id DESC`.
- Deleted event IDs are returned in deletion order oldest-first for predictable
  tests and logs.
- Delete parameters explicitly before deleting event rows. SQLite foreign-key
  cascades are not relied upon because connection-level `PRAGMA foreign_keys`
  may not be enabled by callers.
- Run SQLite cleanup in one transaction per operation.

## Max Bytes

`RetentionConfig.max_bytes` exists, but SQLite file-size enforcement is not a
simple row cleanup because file pages may not shrink without VACUUM or
incremental vacuum settings. Ring buffer byte accounting also does not exist.
This task documents max-bytes as unsupported by current storage primitives and
does not add no-op methods that pretend enforcement happened.

## Compatibility

- Existing insert/query APIs stay unchanged.
- SQLite schema version does not change.
- Runtime storage selection and background scheduling remain out of scope.
