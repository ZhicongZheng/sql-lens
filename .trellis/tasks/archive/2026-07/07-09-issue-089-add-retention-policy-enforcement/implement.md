# Implementation Plan

1. [x] Read backend database/error/quality specs and current storage code.
2. [x] Add `RingBufferRetentionOutcome`.
3. [x] Add `RingBufferStore::enforce_max_events`.
4. [x] Add ring buffer tests for:
   - no-op when current length is within max events;
   - oldest-first deletion when above max events;
   - stats eviction counter update.
5. [x] Add `SqliteRetentionOutcome`.
6. [x] Add `SqliteEventStore::delete_events_older_than`.
7. [x] Add `SqliteEventStore::enforce_max_events`.
8. [x] Ensure SQLite cleanup deletes matching `sql_parameters` rows explicitly.
9. [x] Add SQLite tests for:
   - age cleanup;
   - count cleanup with deterministic `(timestamp DESC, id DESC)` keep set;
   - parameter cleanup;
   - no-op cleanup.
10. [x] Update `STORAGE.md` and backend database spec with the retention
    contract and max-bytes boundary.
11. [x] Run narrow validation: `rtk cargo test -p sql-lens-storage`.
12. [x] Run broad validation:
    - `rtk cargo fmt --check`
    - `rtk cargo test --workspace`
    - `rtk cargo clippy --workspace --all-targets -- -D warnings`
