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

- [x] Query supports limit.
- [x] Query returns reverse chronological order.
- [x] Query returns stable cursors.
- [x] Cursor pagination returns older events without duplicates.
- [x] Cursor stays stable when newer events are appended after the first page.
- [x] Existing append/get/snapshot behavior remains unchanged.
- [x] Tests cover ordering.
- [x] Tests cover limit.
- [x] Tests cover cursor pagination.
- [x] Tests cover cursor stability across append.
- [x] `cargo fmt --check` passes.
- [x] `cargo check --workspace` passes.
- [x] `cargo test --workspace` passes.
- [x] `cargo clippy --workspace --all-targets -- -D warnings` passes.

## Open Questions

None blocking.
