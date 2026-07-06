# Implement ring buffer event lookup

## Goal

Issue 022: support lookup of retained SQL events by `SqlEventId` in the in-memory ring buffer.

## Requirements

- Add lookup to `RingBufferStore`.
- Existing retained events can be retrieved by ID.
- Evicted events return not found.
- Lookup must not mutate storage state or stats.
- Keep implementation minimal; do not add secondary indexes in this task.

## Out Of Scope

- Timeline query.
- Filters.
- Pagination.
- Storage API error types.
- Secondary indexes.
- SQLite lookup.

## Acceptance Criteria

- [x] Existing events can be retrieved.
- [x] Evicted events return not found.
- [x] Lookup returns a borrowed `SqlEvent`.
- [x] Lookup does not mutate stats.
- [x] Tests cover existing event lookup.
- [x] Tests cover evicted event lookup.
- [x] `cargo fmt --check` passes.
- [x] `cargo check --workspace` passes.
- [x] `cargo test --workspace` passes.
- [x] `cargo clippy --workspace --all-targets -- -D warnings` passes.
