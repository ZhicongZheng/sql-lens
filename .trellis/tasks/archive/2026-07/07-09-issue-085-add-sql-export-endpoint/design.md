# Design

## Boundary

Add an `export` module to `sql-lens-api`. The endpoint reuses `SqlEventFilter` from `sql-lens-storage` and the existing query parameter parsing logic from `sql_events.rs`. Redaction uses `redact_sql_event` from `sql-lens-core`.

## Public Contract

```text
GET /api/v1/sql-events/export?format=json|ndjson&<same filters as list endpoint>
```

Query parameters mirror the list endpoint exactly. The additional `format` parameter defaults to `json`.

Response:
- `format=json`: `application/json` body containing a JSON array of `SqlEventDetailResponse` objects.
- `format=ndjson`: `application/x-ndjson` body with one JSON `SqlEventDetailResponse` per line.

## Bounding

`MAX_EXPORT_LIMIT = 10_000`. The endpoint queries the ring buffer with this as the page limit, then serializes the results. No cursor-based pagination is exposed for exports — the caller gets at most `MAX_EXPORT_LIMIT` events in one response.

## Redaction

Each event from the store is passed through `redact_sql_event` with the default policy before serialization. This matches the behavior of the live WebSocket broadcaster.

## Compatibility

This adds a new endpoint. It does not change existing REST response shapes, WebSocket behavior, or storage contracts.
