# Issue 085: Add SQL export endpoint

## Goal

Export filtered SQL events as JSON or NDJSON for offline analysis, debugging handoffs, and tooling integration.

## Background

- `ISSUES.md` Issue 085 requires export that respects filters, redacts events, and bounds large exports.
- The existing `GET /api/v1/sql-events` list endpoint already parses `SqlEventFilter` from query params and returns paginated results.
- `SqlEventDetailResponse` already contains the full event shape.
- `redact_sql_event` in `sql-lens-core` handles sensitive value masking.
- `RingBufferStore::query_timeline` supports filtered pagination.

## Requirements

- Add a `GET /api/v1/sql-events/export` endpoint.
- Accept the same filter query parameters as the list endpoint (target_name, protocol, database_type, database, user, client_addr, status, min_duration_ms, max_duration_ms, q, fingerprint, from, to).
- Support a `format` query parameter: `json` (default) returns a JSON array, `ndjson` returns newline-delimited JSON (one JSON object per line).
- Apply redaction to exported events before serialization.
- Bound export size with a hard maximum limit (e.g., 10,000 events) separate from the list endpoint's paginated limit.
- Invalid filter values return the same `BAD_REQUEST` errors as the list endpoint.

## Acceptance Criteria

- [x] Export endpoint returns filtered events in JSON format.
- [x] Export endpoint returns filtered events in NDJSON format.
- [x] Exported events are redacted.
- [x] Large exports are bounded to a configurable maximum.
- [x] Invalid filters return structured API errors.
- [x] No SQL is executed, no database connection behavior changes.

## Notes

- Out of scope: streaming/chunked response for truly massive exports, file download headers, CSV format, replay execution.
