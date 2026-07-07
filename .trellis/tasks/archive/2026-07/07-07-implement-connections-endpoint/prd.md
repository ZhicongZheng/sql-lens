# Implement connections endpoint

## Goal

Implement Issue 030: add GET /api/v1/connections and GET /api/v1/connections/{id} for connection state.

These endpoints expose the latest retained connection state to the future Connections UI and to local debugging tools.

## Background

- `API.md` defines:
  - `GET /api/v1/connections`
  - `GET /api/v1/connections/{id}`
- Issue 017 added proxy-local `ConnectionLifecycleRecord`, but it did not add an API-readable connection store.
- `sql-lens-core` already owns protocol-neutral `ConnectionInfo` and `ConnectionState`.
- The user confirmed the recommended boundary: add the connection store in `sql-lens-storage`, not as an API-only temporary collection.
- Existing app runtime composition remains out of scope; this task only adds storage/API primitives and tests.

## Requirements

- Add an in-memory connection store to `sql-lens-storage`.
- Store `ConnectionInfo` values by `ConnectionId`.
- Support upsert-style writes so active connection updates and closed connection updates replace the current record.
- Support list-recent queries for connections.
- Support lookup by connection ID.
- Enforce bounded capacity and evict oldest-updated connections when full.
- Add `GET /api/v1/connections`.
- Add `GET /api/v1/connections/{id}`.
- Extend `ApiState` to hold the connection store.
- Keep `ApiState::new(event_store)` compatible by using a default connection store.
- Add a `with_stores(event_store, connection_store)` constructor for tests and future runtime composition.
- Return API DTOs shaped like `API.md`, using lowercase/snake_case state values.
- Return HTTP 404 `NOT_FOUND` for a missing connection.
- Preserve request ID middleware behavior.
- Tests must cover active and closed connections.

## Acceptance Criteria

- [x] Storage can upsert active connection state.
- [x] Storage can upsert closed connection state.
- [x] Storage list returns recent connections newest-first.
- [x] Storage detail lookup returns an existing connection.
- [x] Storage detail lookup returns none for missing/evicted connections.
- [x] `GET /api/v1/connections` returns recent connections.
- [x] `GET /api/v1/connections/{id}` returns detail.
- [x] Missing connection returns HTTP 404 with `NOT_FOUND`.
- [x] Response includes fields documented in `API.md`.
- [x] Tests cover active and closed connections.
- [x] Existing SQL event and health endpoint tests still pass.

## Out Of Scope

- Wiring `sql-lens-proxy` lifecycle updates into the connection store.
- Persistent SQLite/DuckDB connection storage.
- Connection filtering/search.
- Connection pagination cursors.
- Authentication or RBAC.
- Frontend integration.
- Runtime startup changes in `sql-lens-app`.
