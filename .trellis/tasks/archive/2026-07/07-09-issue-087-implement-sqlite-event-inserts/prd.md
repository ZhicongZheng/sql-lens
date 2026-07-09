# Issue 087: Implement SQLite event inserts

## Goal

Persist captured SQL events into SQLite using the schema foundation from Issue 086.

## Background

- Issue 086 added `apply_sqlite_schema` and the initial SQLite tables/indexes.
- `RingBufferStore::append` already applies redaction before retention.
- `STORAGE.md` requires SQLite persistence to store already-redacted protocol-neutral events.
- Issue 088 will add timeline queries, so this task should include only the insert/readback foundation needed to prove event persistence.

## Requirements

- Add a SQLite event insert API in `sql-lens-storage`.
- Insert one `SqlEvent` into `sql_events` and its parameters into `sql_parameters`.
- Store protocol-neutral scalar fields directly and structured metadata/parameter values as JSON text.
- Ensure stored events are redacted before writing, matching ring buffer behavior.
- Provide focused tests using in-memory SQLite and `apply_sqlite_schema`.
- Keep app runtime/config wiring out of scope.

## Acceptance Criteria

- [ ] Inserts are asynchronous or buffered, or the sync API is clearly storage-local and not wired into capture runtime.
- [ ] Redacted events are stored.
- [ ] Tests cover insert and readback.
- [ ] Inserts include `sql_events` and `sql_parameters` rows.
- [ ] No API, proxy, or app runtime behavior changes.

## Notes

- Out of scope: SQLite timeline query API, retention cleanup, async writer task, config selection, file lifecycle management, and runtime fan-out from capture channel.
