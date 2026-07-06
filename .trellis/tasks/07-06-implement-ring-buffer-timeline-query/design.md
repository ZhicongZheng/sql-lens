# Implement ring buffer timeline query design

## Boundary

Implement in `crates/sql-lens-storage`.

No new dependencies.

## Public API

Planned types:

- `RingBufferTimelineQuery`
  - `limit: NonZeroUsize`
  - `cursor: Option<RingBufferTimelineCursor>`
- `RingBufferTimelineCursor`
  - `before_sequence: u64`
- `RingBufferTimelinePage`
  - `events: Vec<SqlEvent>`
  - `next_cursor: Option<RingBufferTimelineCursor>`

Add:

```rust
impl RingBufferStore {
    pub fn query_timeline(&self, query: RingBufferTimelineQuery) -> RingBufferTimelinePage;
}
```

## Cursor Semantics

Internally, each appended event receives a monotonically increasing `sequence`.

Timeline query behavior:

- No cursor means return retained events with any sequence, newest first.
- A cursor with `before_sequence = N` means return retained events with sequence `< N`, newest first.
- The next cursor points to the sequence of the oldest event returned in the current page, but only when older retained events still exist.

This cursor is stable across new appends because newer events receive larger sequences and are excluded by the next page cursor.

## Storage Shape

Change the internal ring buffer from `VecDeque<SqlEvent>` to `VecDeque<RingBufferEntry>`.

`RingBufferEntry` contains:

- `sequence: u64`
- `event: SqlEvent`

Public APIs still expose `SqlEvent` only.

## Tests

- Query returns newest first.
- Limit truncates results.
- Cursor returns the next older page without duplicates.
- Cursor page ignores newer events appended after the first page.
- Existing snapshot remains oldest-to-newest.
