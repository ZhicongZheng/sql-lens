# Issue 089: Add retention policy enforcement

## Goal

Add storage-local retention enforcement primitives so SQL Lens can trim retained
SQL events by maximum event count and, for SQLite, by timestamp cutoff.

## Background

- `RetentionConfig` already exists in `sql-lens-config` with `max_age`,
  `max_events`, `max_bytes`, and `drop_policy`.
- `RingBufferStore` already enforces its fixed capacity during append, but it
  does not expose a cleanup API for applying a smaller max-events policy to
  already-retained events.
- `SqliteEventStore` supports redacted inserts and timeline queries, but it
  does not remove old persisted events.
- `STORAGE.md` states retention dimensions are max event count, max age, and max
  storage bytes, with retention applied after redaction.
- Issue 089 requires ring buffer max-events behavior, SQLite age and event-count
  cleanup, and cleanup tests.

## Requirements

- Keep retention enforcement inside `sql-lens-storage`.
- Add a ring-buffer max-events cleanup API that evicts oldest retained events
  when the policy is lower than the current retained length.
- Add SQLite retention cleanup APIs for:
  - deleting events older than a caller-provided timestamp cutoff;
  - deleting oldest events when persisted count exceeds a caller-provided max
    event count.
- SQLite cleanup must remove matching `sql_parameters` rows for deleted events.
- Return structured cleanup outcomes including deleted event IDs and counts.
- Keep runtime config parsing/wiring, background cleanup scheduling, API
  endpoints, frontend changes, and SQLite file-size/VACUUM behavior out of
  scope.
- Treat `max_bytes` as not supported by current storage primitives; do not add a
  fake byte-enforcement implementation.

## Acceptance Criteria

- [ ] Ring buffer can enforce a lower max-events policy against already-retained
      events.
- [ ] Ring buffer cleanup evicts oldest events first and reports deleted IDs.
- [ ] SQLite cleanup deletes events older than a timestamp cutoff.
- [ ] SQLite cleanup enforces max event count by deleting oldest events according
      to SQLite timeline ordering.
- [ ] SQLite cleanup removes parameters for deleted events.
- [ ] Tests cover ring buffer max-events cleanup and SQLite age/count cleanup.
- [ ] No API/proxy/app runtime/frontend behavior changes.

## Notes

- This task is a storage foundation. A later runtime wiring task can translate
  `RetentionConfig.max_age` and `RetentionConfig.max_events` into these storage
  APIs.
