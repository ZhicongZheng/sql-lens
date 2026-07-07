# Implement SQL event list endpoint

## Goal

Implement Issue 028: add GET /api/v1/sql-events reading from storage with query parameters mapped to storage filters and response matching API.md.

This endpoint powers the SQL timeline list in the future UI. It should expose retained SQL events from the ring buffer through the API layer with filtering, pagination, and API-shaped DTOs instead of leaking internal storage structs directly.

## Background

- `API.md` defines `GET /api/v1/sql-events`.
- Issue 024 added storage timeline filters for `protocol`, `database_type`, `database`, `user`, `status`, duration range, text search, and timestamp range.
- Issue 024 intentionally left `client_addr` and `fingerprint` filters for a future task even though `API.md` lists them.
- The user confirmed this task should also fill the `client_addr` and `fingerprint` filter gap.
- Issue 026 added the HTTP server foundation.
- Issue 027 added `GET /api/v1/health`.
- Current `sql-lens-app` still does not compose runtime services; this task should stay inside API/storage primitives and tests.

## Requirements

- Add `client_addr` and `fingerprint` fields to `sql-lens-storage::SqlEventFilter`.
- Extend ring buffer timeline filtering to match `client_addr` exactly and `fingerprint` exactly.
- Add `GET /api/v1/sql-events` to `sql-lens-api`.
- Endpoint must read retained events from `RingBufferStore`.
- Provide API state that can hold a ring buffer store for tests and future runtime composition.
- Support query parameters:
  - `limit`
  - `cursor`
  - `protocol`
  - `database_type`
  - `database`
  - `user`
  - `client_addr`
  - `status`
  - `min_duration_ms`
  - `max_duration_ms`
  - `q`
  - `fingerprint`
  - `from`
  - `to`
- Map supported query parameters to `RingBufferTimelineQuery` and `SqlEventFilter`.
- Use a deterministic cursor encoding for ring buffer timeline pagination.
- Return list DTOs shaped for `API.md`, including item summary fields and `next_cursor`.
- Return metadata as a protocol-keyed object derived from `ProtocolMetadata`.
- Return `rows` from `ResultSummary` when result data exists.
- Preserve request ID middleware behavior on list responses and errors.
- Return `BAD_REQUEST` style API errors for invalid query/filter input that this endpoint validates.
- Keep this task out of runtime composition: do not start storage from `sql-lens-app`.

## Acceptance Criteria

- [x] Storage filters support `client_addr`.
- [x] Storage filters support `fingerprint`.
- [x] `GET /api/v1/sql-events` returns retained events from API state storage.
- [x] Query parameters map to storage filters.
- [x] `limit` constrains page size.
- [x] `cursor` requests older pages without duplicating events.
- [x] `next_cursor` is returned when older matching events exist.
- [x] Response items include fields documented by `API.md`.
- [x] Metadata is returned in a protocol-keyed JSON object.
- [x] Invalid duration ranges return HTTP 400.
- [x] Invalid cursor returns HTTP 400.
- [x] Endpoint tests cover response schema and at least one filtered query.
- [x] Existing health, request ID, storage, and workspace tests still pass.

## Out Of Scope

- `GET /api/v1/sql-events/{id}` detail endpoint.
- WebSocket SQL stream.
- SQLite or DuckDB-backed API reads.
- Runtime composition in `sql-lens-app`.
- OpenAPI file generation.
- Authentication, RBAC, CORS, TLS, or frontend integration.
- Case-insensitive SQL search.
- Multi-value status filters; REST list accepts a single `status` value in this task.

## Notes

- Keep `prd.md` focused on requirements, constraints, and acceptance criteria.
- Lightweight tasks can remain PRD-only.
- For complex tasks, add `design.md` for technical design and `implement.md` for execution planning before `task.py start`.
