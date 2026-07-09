# Issue 086: Add SQLite storage schema design

## Goal

Implement the first SQLite schema and migration table so SQL Lens can create an empty persistent storage database that matches `STORAGE.md`.

## Background

- `STORAGE.md` defines optional SQLite persistence with tables for SQL events, parameters, connections, prepared statements, and schema version.
- Current `sql-lens-storage` only has in-memory ring buffer, connection store, and live statistics.
- Follow-up issues depend on this schema: Issue 087 inserts, Issue 088 timeline queries, and Issue 089 retention.

## Requirements

- Add a SQLite schema module in `sql-lens-storage`.
- Define the initial migration SQL for an empty SQLite database.
- Create tables matching `STORAGE.md`: `schema_version`, `sql_events`, `sql_parameters`, `connections`, `prepared_statements`.
- Create indexes recommended by `STORAGE.md` for timeline and common filters.
- Provide a small migration API that applies the initial schema to an empty database.
- Keep runtime capture path unchanged; this task only adds schema/migration foundation.

## Acceptance Criteria

- [x] Tables match `STORAGE.md`.
- [x] Schema version table exists.
- [x] Migration can be applied to an empty database.
- [x] Tests verify tables and indexes exist after migration.
- [x] No ring buffer, API, or proxy behavior changes.

## Notes

- Out of scope: event inserts, SQLite timeline queries, retention cleanup, config wiring, async buffering, file lifecycle management, and API selection between storage backends.
