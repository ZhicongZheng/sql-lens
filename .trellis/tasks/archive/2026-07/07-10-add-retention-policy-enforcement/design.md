# Retention Policy Enforcement Design

## Architecture

### Components

1. **Storage Layer Cleanup Methods** (existing)
   - `RingBufferStore::enforce_max_events()` — in-memory eviction of oldest events
   - `SqliteEventStore::delete_events_older_than()` — age-based deletion with transaction
   - `SqliteEventStore::enforce_max_events()` — count-based deletion of oldest events

2. **Retention Configuration** (existing, not yet wired)
   - `RetentionConfig` in `sql-lens-config` provides global defaults
   - `max_age`, `max_events`, `max_bytes`, `drop_policy` fields

3. **App Runtime Integration** (deferred to Issue 117)
   - Reads `RetentionConfig` from app config
   - Calls storage cleanup methods during capture or as background task
   - Not part of Issue 089 scope

### Data Flow

```
RetentionConfig (global) → [Future: App Runtime] → Storage.cleanup(max_events/age)
                                                          ↓
                                              RingBufferStore / SqliteEventStore
```

### Boundaries

- **Issue 089 scope**: Storage layer methods + test coverage
- **Issue 117 scope**: App runtime reads config, calls cleanup methods
- **Future scope**: Per-table overrides, async scheduling, max_bytes support

## Contracts

### Ring Buffer Cleanup

```rust
impl RingBufferStore {
    /// Drop oldest events until `events.len() <= max_events`.
    /// Returns deleted event IDs for audit/logging.
    pub fn enforce_max_events(&mut self, max_events: NonZeroUsize) -> RingBufferRetentionOutcome;
}

pub struct RingBufferRetentionOutcome {
    pub deleted_event_ids: Vec<SqlEventId>,
}
```

### SQLite Cleanup

```rust
impl SqliteEventStore {
    /// Delete all events with timestamp < cutoff.
    /// Deletes associated parameter rows via transaction.
    pub fn delete_events_older_than(&mut self, cutoff: &Timestamp) -> rusqlite::Result<SqliteRetentionOutcome>;

    /// Delete oldest events until total count <= max_events.
    /// Preserves timeline ordering (ORDER BY timestamp ASC, id ASC).
    pub fn enforce_max_events(&mut self, max_events: NonZeroUsize) -> rusqlite::Result<SqliteRetentionOutcome>;
}

pub struct SqliteRetentionOutcome {
    pub deleted_event_ids: Vec<SqlEventId>,
    // Additional metrics may be added
}
```

## Trade-offs

| Decision | Rationale |
|----------|-----------|
| Synchronous cleanup methods | Simpler API, caller controls execution context (sync vs async) |
| Global-only retention for Issue 089 | Matches AC scope; per-table overrides add significant complexity |
| No max_bytes implementation | Explicitly unsupported per AC; requires separate file-size cleanup design |

## Compatibility & Migration

- Storage layer methods are additive — no breaking changes to existing APIs
- Existing unit tests (`ring_buffer_retention_*`, `sqlite_retention_*`) validate behavior
- Future app runtime integration (Issue 117) will consume these methods without modification

## Operational Considerations

- Cleanup methods use transactions (SQLite) or direct mutation (ring buffer) — no external coordination needed
- Error handling: SQLite methods return `rusqlite::Result`, ring buffer is infallible
- Performance: Ring buffer O(n) eviction, SQLite O(delete_count) with index on timestamp
