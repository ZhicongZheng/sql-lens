# Implement SQL event detail endpoint

## Goal

Implement Issue 029: add GET /api/v1/sql-events/{id}, returning full retained event detail or NOT_FOUND.

This endpoint lets the future UI open a selected SQL timeline row and inspect the complete captured event, including parameters, timings, result summary, error summary, and protocol metadata.

## Background

- `API.md` defines `GET /api/v1/sql-events/{id}`.
- Issue 022 added `RingBufferStore::get(&SqlEventId)`.
- Issue 026 added the HTTP server foundation.
- Issue 028 added `ApiState`, `router_with_state`, and the SQL event list DTO/error infrastructure.
- This task should extend the same `sql_events` API module rather than creating a second parallel endpoint stack.

## Requirements

- Add `GET /api/v1/sql-events/{id}` to `sql-lens-api`.
- Look up retained events from `ApiState`'s `RingBufferStore`.
- Return HTTP 200 with a full event detail DTO when the event exists.
- Return HTTP 404 with documented `NOT_FOUND` API error shape when the event is missing.
- Include all list-summary fields.
- Include `normalized_sql`.
- Include `parameters`.
- Include `timings`.
- Include `result`.
- Include `error`.
- Include protocol-keyed `metadata`.
- Preserve request ID middleware on success and error responses.
- Reuse list endpoint mapping helpers where practical.
- Do not change `sql-lens-app` runtime behavior.

## Acceptance Criteria

- [x] Existing event returns HTTP 200.
- [x] Missing event returns HTTP 404.
- [x] Missing event response uses `NOT_FOUND` error code.
- [x] Detail response includes parameters.
- [x] Detail response includes timings.
- [x] Detail response includes result summary.
- [x] Detail response includes error summary when present.
- [x] Detail response includes protocol-keyed metadata.
- [x] Detail response preserves request ID header.
- [x] Existing SQL event list tests still pass.
- [x] Full workspace validation passes.

## Out Of Scope

- Persistent storage lookup.
- Detail endpoint authorization.
- Replay actions from the detail view.
- SQL formatting or syntax highlighting.
- Runtime composition in `sql-lens-app`.
