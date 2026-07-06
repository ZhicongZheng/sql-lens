# Implement in-memory ring buffer append design

## Boundary

Implement in `crates/sql-lens-storage`.

Allowed dependency:

- `sql-lens-core`

Do not add async runtime, database, API, protocol, app, or serialization dependencies.

## Public API

Planned types:

- `RingBufferStore`
  - `new(capacity: NonZeroUsize) -> Self`
  - `append(event: SqlEvent) -> RingBufferAppendOutcome`
  - `snapshot() -> Vec<SqlEvent>`
  - `stats() -> RingBufferStats`
  - `len()`, `capacity()`, `is_empty()`
- `RingBufferAppendOutcome`
  - `stored_event_id`
  - `evicted_event_id`
- `RingBufferStats`
  - `capacity`
  - `len`
  - `total_appended`
  - `total_evicted`

## Storage Shape

Use `VecDeque<SqlEvent>`.

When append is called:

1. If current length equals capacity, pop one front event and count it as evicted.
2. Push the incoming event to the back.
3. Increment total appended.
4. Return IDs for stored and evicted events.

## Why No Lookup Yet

Issue 022 owns lookup by ID. This task should avoid adding an index until lookup requirements are implemented.

## Tests

- Append to an empty ring buffer.
- Fill to capacity.
- Append past capacity and assert oldest event is gone.
- Assert stats after append and eviction.
- Assert zero capacity is impossible through `NonZeroUsize`.
