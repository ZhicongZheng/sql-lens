# Implement ring buffer timeline query

## Goal

Issue 023: query retained ring buffer events in reverse chronological order.

## User Value

The SQL timeline page and API need a storage primitive that can return recent events first, with bounded result size and a cursor that can be used for the next page.

## Requirements

- Add timeline query support to `RingBufferStore`.
- Query returns events in reverse insertion order, newest first.
- Query supports a non-zero limit.
- Query returns a cursor when more older retained events are available.
- Cursor must be stable across new appends.
- Evicted events may naturally disappear from later cursor pages.
- Existing append, snapshot, lookup, and stats behavior must remain unchanged.
- Add tests for ordering, limit, cursor paging, and cursor stability across append.

## Out Of Scope

- Filtering by SQL text/status/duration.
- API pagination schema.
- Timestamp parsing.
- Persistent cursor serialization.
- SQLite/DuckDB queries.
- Secondary indexes.

## Acceptance Criteria

- [ ] Query supports limit.
- [ ] Query returns reverse chronological order.
- [ ] Query returns stable cursors.
- [ ] Cursor pagination returns older events without duplicates.
- [ ] Cursor stays stable when newer events are appended after the first page.
- [ ] Existing append/get/snapshot behavior remains unchanged.
- [ ] Tests cover ordering.
- [ ] Tests cover limit.
- [ ] Tests cover cursor pagination.
- [ ] Tests cover cursor stability across append.
- [ ] `cargo fmt --check` passes.
- [ ] `cargo check --workspace` passes.
- [ ] `cargo test --workspace` passes.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes.

## Open Questions

None blocking.
