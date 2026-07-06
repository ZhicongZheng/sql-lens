# Implement storage filters design

## Boundary

Implement filter support inside `crates/sql-lens-storage`.

No new dependencies.

This task extends the ring buffer timeline query only. It does not introduce API query parsing, WebSocket filters, SQLite/DuckDB query support, secondary indexes, SQL parsing, or timestamp parsing.

## Public API

Extend `RingBufferTimelineQuery` with a filter field:

```rust
pub struct RingBufferTimelineQuery {
    pub limit: NonZeroUsize,
    pub cursor: Option<RingBufferTimelineCursor>,
    pub filter: SqlEventFilter,
}
```

Add storage filter types:

```rust
pub struct SqlEventFilter {
    pub protocol: Option<ProtocolName>,
    pub database_type: Option<DatabaseType>,
    pub database: Option<String>,
    pub user: Option<String>,
    pub status: Option<CaptureStatus>,
    pub min_duration: Option<DurationMillis>,
    pub max_duration: Option<DurationMillis>,
    pub text: Option<String>,
    pub from: Option<Timestamp>,
    pub to: Option<Timestamp>,
}

pub enum SqlEventFilterError {
    InvalidDurationRange {
        min: DurationMillis,
        max: DurationMillis,
    },
    InvalidTimestampRange {
        from: Timestamp,
        to: Timestamp,
    },
}
```

Change timeline query to return a `Result`:

```rust
impl RingBufferStore {
    pub fn query_timeline(
        &self,
        query: RingBufferTimelineQuery,
    ) -> Result<RingBufferTimelinePage, SqlEventFilterError>;
}
```

`SqlEventFilter::default()` represents no filters.

## Matching Semantics

- All non-empty filter fields are combined with logical AND.
- `protocol` matches `SqlEvent.protocol` exactly.
- `database_type` matches `SqlEvent.database_type` exactly.
- `database` matches `SqlEvent.database.as_deref()` exactly.
- `user` matches `SqlEvent.user.as_deref()` exactly.
- `status` matches `SqlEvent.status` exactly.
- `min_duration` keeps events with `duration >= min_duration`.
- `max_duration` keeps events with `duration <= max_duration`.
- `text` performs a case-sensitive substring match against:
  - `original_sql`
  - `normalized_sql`
  - `expanded_sql`
- `from` keeps events with `timestamp >= from`.
- `to` keeps events with `timestamp <= to`.

## Validation Semantics

Storage filters are strongly typed. Unknown API query parameters do not reach storage and will be rejected by the future API layer.

Storage validates only supported typed filters:

- `min_duration > max_duration` returns `SqlEventFilterError::InvalidDurationRange`.
- `from > to` returns `SqlEventFilterError::InvalidTimestampRange`.

The filter is validated before scanning retained entries.

## Cursor Semantics

Cursor behavior remains sequence-based:

- No cursor means start from the newest retained event.
- `before_sequence = N` means only entries with internal sequence `< N`.
- Newer appends after a cursor is issued do not affect the older page.

Filters are applied while scanning retained entries. `next_cursor` is returned only when an older retained entry matching the same filter exists after the current page.

## Time Range Note

`Timestamp` is currently a string newtype. This task compares timestamp strings through the existing `Ord` implementation and assumes captured timestamps use a stable sortable representation such as RFC 3339 / ISO 8601 UTC.

No timestamp parsing or normalization is added in this task.

## Trade-Offs

- Linear scan keeps the implementation simple and is acceptable for the current in-memory ring buffer.
- Secondary indexes can be added later when filter performance requirements are measured.
- Case-sensitive text search avoids allocation and locale rules for now; case-insensitive search can be a later explicit product decision.
- Fingerprint and client address filters are left for a future task because Issue 024 does not require them, even though API/PRD mention them.

## Compatibility

Existing `query_timeline` callers must provide `SqlEventFilter::default()` and handle `Result`.

Existing append, lookup, snapshot, stats, and event retention behavior must remain unchanged.
