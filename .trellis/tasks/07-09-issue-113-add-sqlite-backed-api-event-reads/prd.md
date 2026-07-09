# Issue 113: Add SQLite-backed API event reads

## Goal

Read persisted SQL event timeline and detail data from SQLite when SQLite storage is configured.

## Background

- Issue 088 added storage-local SQLite timeline queries.
- Issue 112 wired configured SQLite persistence into app runtime while keeping API reads on the live ring buffer.
- Local demo data now survives in `sql-lens.local.sqlite3`, but `/api/v1/sql-events`, SQL event detail, export, and replay preview still look only at the in-memory ring buffer.
- The next backend step is a configured read-source boundary so local users can restart the app and still inspect persisted captured SQL.

## Requirements

- Add an API-owned event read abstraction or equivalent small boundary that supports the existing SQL event list/detail/read consumers.
- Preserve default ring-buffer-only behavior when SQLite storage is not configured.
- When SQLite storage is configured, read persisted SQL event timeline and detail data from SQLite for:
  - `GET /api/v1/sql-events`
  - `GET /api/v1/sql-events/{id}`
  - `GET /api/v1/sql-events/export`
  - `POST /api/v1/replay/preview` when `event_id` is provided
- Keep existing REST response schemas and query parameters stable.
- Map SQLite read failures into the existing API error envelope.
- Support persisted event parameters in SQL event detail responses.
- Keep packet forwarding and SQLite writes non-blocking; this task must not put synchronous SQLite reads on the proxy hot path.

## Out Of Scope

- Frontend changes.
- Replay execute.
- SQLite writes, migrations, or retention policy changes.
- DuckDB or analytics storage.
- SQL rewriting or database connection behavior.
- Authentication, RBAC, or API pagination redesign.

## Acceptance Criteria

- [x] Default ring-buffer API reads continue to pass existing list/detail/export/replay tests.
- [x] SQLite-backed list reads return persisted events with existing filters and cursor semantics.
- [x] SQLite-backed detail reads return persisted event fields and parameters.
- [x] SQLite-backed export returns persisted redacted event details.
- [x] SQLite-backed replay preview by event ID uses the persisted event SQL.
- [x] SQLite read errors are surfaced through the existing API error envelope.
- [x] App runtime selects SQLite-backed API reads when `storage.type = "sqlite"`.
- [x] `rtk cargo fmt --check` passes.
- [x] `rtk cargo test -p sql-lens-storage` passes if storage mapping code changes.
- [x] `rtk cargo test -p sql-lens-api` passes.
- [x] `rtk cargo test -p sql-lens-app` passes.
- [x] `rtk cargo test --workspace` passes.
- [x] `rtk cargo clippy --workspace --all-targets -- -D warnings` passes.

## Notes

- Keep implementation direct. Prefer a narrow API read-source boundary over a broad storage abstraction.
- If full reconstruction of every future `SqlEvent` field from SQLite becomes too large, preserve the current REST DTO contract rather than expanding core models.
