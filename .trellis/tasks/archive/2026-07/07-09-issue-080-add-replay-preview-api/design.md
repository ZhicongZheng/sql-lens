# Design — Issue 080: Replay Preview API

## Boundary

This task adds a preview-only REST endpoint. It does not execute SQL, dial a
backend, create replay jobs, or alter proxy traffic.

## API Shape

Endpoint:

```http
POST /api/v1/replay/preview
```

Request shape should be explicit and simple:

- `event_id`: optional captured SQL event ID.
- `sql`: optional raw SQL payload.

Exactly one source should be provided. Supplying neither or both should return
`BAD_REQUEST`.

Response shape should include:

- `source`: `event` or `raw_sql`.
- `event_id`: present for event-source previews.
- `sql`: final preview SQL.
- `is_mutation`: boolean.
- `warning`: optional warning text for mutating SQL.

## SQL Selection

For captured events, choose:

1. `expanded_sql` when present.
2. `original_sql` otherwise.

This matches existing prepared statement rendering behavior while avoiding any
new SQL reconstruction logic in the API layer.

## Mutation Classification

Implement a small replay-local classifier for the first SQL keyword after
comments/whitespace are ignored enough for common local usage.

Initial read-only keywords:

- `SELECT`
- `SHOW`
- `DESCRIBE`
- `DESC`
- `EXPLAIN`

Everything else should be treated as mutating or potentially mutating. This is
intentionally conservative for safety.

## Error Handling

- Missing event ID: `NOT_FOUND`, field `event_id`.
- Missing both sources: `BAD_REQUEST`, field `source`.
- Supplying both sources: `BAD_REQUEST`, field `source`.
- Empty raw SQL or event SQL: `BAD_REQUEST`, field `sql`.

Use existing `ApiEndpointError` helpers and request ID middleware.

## Ownership

- `sql-lens-api` owns the REST endpoint, request/response DTOs, and preview
  classification.
- Storage remains read-only from this endpoint.
- Core models are reused; no new core dependency or protocol-specific field is
  needed.
