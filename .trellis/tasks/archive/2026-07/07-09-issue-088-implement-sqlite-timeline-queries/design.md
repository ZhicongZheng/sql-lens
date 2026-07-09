# Design

## Boundary

Implement SQLite timeline querying inside `sql-lens-storage`, extending
`SqliteEventStore`. Do not change API handlers, app startup, proxy forwarding,
capture fan-out, or frontend contracts in this task.

## Public Contract

Add SQLite-owned query/page/cursor types that mirror ring buffer semantics
without pretending the cursor sequence is the same storage key:

```rust
pub struct SqliteTimelineQuery {
    pub limit: NonZeroUsize,
    pub cursor: Option<SqliteTimelineCursor>,
    pub filter: SqlEventFilter,
}

pub struct SqliteTimelineCursor {
    pub before_timestamp: Timestamp,
    pub before_event_id: SqlEventId,
}

pub struct SqliteTimelinePage {
    pub events: Vec<SqliteEventRow>,
    pub next_cursor: Option<SqliteTimelineCursor>,
}

impl SqliteEventStore {
    pub fn query_timeline(
        &self,
        query: SqliteTimelineQuery,
    ) -> Result<SqliteTimelinePage, SqliteTimelineQueryError>;
}
```

The page returns `SqliteEventRow` because Issue 087's readback shape already
matches the persisted scalar columns. Full reconstruction into `SqlEvent`
requires parsing metadata and parameter values back into core domain types and
is not needed for this issue's timeline foundation.

## Ordering And Cursor

SQLite rows are ordered by `(timestamp DESC, id DESC)`.

The cursor means "return rows older than this row in that ordering":

```sql
WHERE (timestamp < ?)
   OR (timestamp = ? AND id < ?)
ORDER BY timestamp DESC, id DESC
LIMIT ?
```

This gives deterministic ordering when multiple events share a timestamp and
keeps already-issued cursors stable if newer events are inserted later.

## Filters

Use `SqlEventFilter::validate` semantics before building SQL. The existing
method is private, so implementation can either make it `pub(crate)` or add a
public storage-level validator helper without exposing it outside the crate.

Indexed/common filters should become SQL predicates:

- `target_name`
- `protocol`
- `database_type`
- `database_name`
- `user_name`
- `client_addr`
- `status`
- `duration_ms >=`
- `duration_ms <=`
- `fingerprint`
- `timestamp >=`
- `timestamp <=`

SQL text search uses the same ring buffer semantics across `original_sql`,
`normalized_sql`, and `expanded_sql`. It can use `LIKE` for now; FTS remains out
of scope.

## Error Handling

The public query method should preserve filter validation parity while also
surfacing SQLite failures. Do not collapse SQLite errors into
`SqlEventFilterError`.

Use this minimal shape:

```rust
pub enum SqliteTimelineQueryError {
    InvalidFilter(SqlEventFilterError),
    Sqlite(rusqlite::Error),
}
```

Implement `From<SqlEventFilterError>` and `From<rusqlite::Error>` plus
`Display`/`Error`.

## Compatibility

- Existing ring buffer query types and API behavior stay unchanged.
- SQLite schema version should not change; Issue 086 already created the needed
  columns and indexes.
- Runtime storage selection remains out of scope.
