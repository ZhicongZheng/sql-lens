# Implementation Plan

1. [x] Read backend database/error/quality specs and existing storage code.
2. [x] Add SQLite timeline public types:
   - `SqliteTimelineQuery`
   - `SqliteTimelineCursor`
   - `SqliteTimelinePage`
   - `SqliteTimelineQueryError`
3. [x] Add `SqliteEventStore::query_timeline`.
4. [x] Build SQL predicates from `SqlEventFilter` with bound parameters.
5. [x] Use deterministic ordering by `(timestamp DESC, id DESC)`.
6. [x] Compute `next_cursor` from the oldest returned row only when there are
   more older rows. Use `limit + 1` internally to detect more rows.
7. [x] Reuse existing row-mapping helper for `SqliteEventRow`.
8. [x] Re-export the new SQLite timeline types from `sql-lens-storage`.
9. [x] Add focused in-memory SQLite tests:
   - newest-first ordering
   - limit and next cursor
   - multi-page cursor without duplicates
   - cursor stable after newer insert
   - target/protocol/status/database/user filters
   - duration/timestamp/text/fingerprint filters
   - invalid range errors
   - empty final page / no next cursor
10. [x] Update `STORAGE.md` and backend database spec if the final contract
    differs from this plan.
11. [x] Run narrow validation: `rtk cargo test -p sql-lens-storage`.
12. [x] Run broad validation:
    - `rtk cargo fmt --check`
    - `rtk cargo test --workspace`
    - `rtk cargo clippy --workspace --all-targets -- -D warnings`
