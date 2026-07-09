# Issue 088: Implement SQLite timeline queries

## Goal

Add storage-local timeline querying for persisted SQLite SQL events so the
SQLite backend can read recently captured history with the same ordering,
pagination, and filter semantics as the in-memory ring buffer.

## Background

- Issue 086 added the SQLite schema and indexes.
- Issue 087 added `SqliteEventStore`, redacted event inserts, parameter inserts,
  and test/support readback helpers.
- `RingBufferStore::query_timeline` owns the current timeline behavior:
  newest-first pages, cursor-based pagination over older events, stable cursors
  when newer events are inserted, shared `SqlEventFilter` validation, and common
  filters.
- `ISSUES.md` describes Issue 088 as querying persisted SQL events from SQLite
  with filters. Acceptance requires ring-buffer behavior parity, indexed common
  filters, and pagination tests.

## Requirements

- Add a SQLite timeline query API in `sql-lens-storage`.
- Query persisted `sql_events` from SQLite newest-first.
- Support cursor-based pagination over older persisted events.
- Reuse or mirror the existing storage filter contract:
  - `target_name`
  - `protocol`
  - `database_type`
  - `database`
  - `user`
  - `client_addr`
  - `status`
  - `min_duration`
  - `max_duration`
  - `text`
  - `fingerprint`
  - `from`
  - `to`
- Validate invalid duration and timestamp ranges before querying.
- Return enough event fields for timeline consumers; full domain reconstruction
  from SQLite parameters can stay out of scope until a later detail/replay task.
- Keep API, proxy, app runtime config, retention cleanup, and frontend behavior
  out of scope.

## Acceptance Criteria

- [ ] SQLite timeline query returns newest events first.
- [ ] SQLite cursor pages older events without duplicates.
- [ ] SQLite cursor remains stable when newer events are inserted after a page is
      read.
- [ ] SQLite timeline filters match ring buffer behavior for common indexed
      fields and SQL text/fingerprint.
- [ ] Invalid duration and timestamp ranges are reported through the existing
      `SqlEventFilterError` inside the SQLite query error type.
- [ ] Tests cover pagination, filters, empty result, and missing next cursor on
      the final page.
- [ ] No API/proxy/app runtime behavior changes.

## Notes

- This task should stay synchronous and storage-local, matching Issue 087's
  current `SqliteEventStore` boundary.
- A later runtime wiring task can decide how SQLite query APIs are selected by
  the application.
