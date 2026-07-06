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

- [ ] Existing events can be retrieved.
- [ ] Evicted events return not found.
- [ ] Lookup returns a borrowed `SqlEvent`.
- [ ] Lookup does not mutate stats.
- [ ] Tests cover existing event lookup.
- [ ] Tests cover evicted event lookup.
- [ ] `cargo fmt --check` passes.
- [ ] `cargo check --workspace` passes.
- [ ] `cargo test --workspace` passes.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes.
