# Implement in-memory ring buffer append

## Goal

Issue 021: implement append-only storage for `SqlEvent` values in a fixed-size in-memory ring buffer.

## User Value

SQL Lens needs a default local storage backend before timeline, lookup, statistics, and API endpoints can be built. The first storage primitive should append events quickly and bound memory by evicting the oldest events.

## Background

- `sql-lens-core` owns `SqlEvent`.
- `STORAGE.md` defines the ring buffer as the default storage backend.
- Later issues add event lookup, timeline query, filters, retention, and SQLite.

## Requirements

- Implement ring buffer append in `sql-lens-storage`.
- Store `SqlEvent` values in insertion order.
- Enforce fixed capacity.
- Default eviction policy is oldest-first.
- Appending while full evicts exactly one oldest event.
- Track basic stats:
  - configured capacity
  - current length
  - total appended events
  - total evicted events
- Expose a snapshot method for tests and future timeline implementation.
- Capacity must be non-zero.
- Add unit tests for append, capacity enforcement, oldest eviction, and stats.

## Out Of Scope

- Lookup by event ID.
- Timeline query pagination.
- Filters.
- Retention by age/bytes.
- SQLite/DuckDB.
- Async storage writer.
- Thread-safe wrapper.
- Config wiring.

## Acceptance Criteria

- [ ] `sql-lens-storage` depends on `sql-lens-core`.
- [ ] Events can be appended.
- [ ] Capacity is enforced.
- [ ] Oldest events are evicted by default.
- [ ] Append outcome reports whether an event was evicted.
- [ ] Stats track capacity, current length, total appended, and total evicted.
- [ ] Zero capacity cannot construct a ring buffer.
- [ ] Tests cover append.
- [ ] Tests cover capacity enforcement.
- [ ] Tests cover oldest eviction.
- [ ] Tests cover stats.
- [ ] `cargo fmt --check` passes.
- [ ] `cargo check --workspace` passes.
- [ ] `cargo test --workspace` passes.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes.

## Open Questions

None blocking.
