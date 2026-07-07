# Implement statistics endpoint

## Goal

Implement Issue 031: add `GET /api/v1/statistics` so the API can expose the current live dashboard summary: QPS, error rate, slow SQL count, latency percentiles, and active connection count.

## Background

- `API.md` documents `GET /api/v1/statistics` with a `window` parameter and a response containing `qps`, `error_rate`, `slow_count`, `latency_ms.p50/p95/p99`, and `active_connections`.
- `sql-lens-storage` already has `LiveStatistics` counters for total events, error events, slow events, fixed 60-second QPS, latency buckets, and active connection count.
- The existing live statistics design intentionally deferred percentile calculation to later API/statistics work. This issue is that follow-up.

## Requirements

- Add `GET /api/v1/statistics`.
- Return the documented JSON fields:
  - `window`
  - `qps`
  - `error_rate`
  - `slow_count`
  - `latency_ms.p50`
  - `latency_ms.p95`
  - `latency_ms.p99`
  - `active_connections`
- Validate `window`.
- For the first API version, support the live in-memory statistics window only.
- Preserve protocol-neutral API behavior; do not add MySQL-specific fields.
- Keep the endpoint read-only.
- Keep storage and API changes small and deterministic.

## Acceptance Criteria

- [ ] `GET /api/v1/statistics` returns HTTP 200 with the documented response shape for an empty state.
- [ ] Populated live statistics produce non-zero QPS, error rate, slow count, latency percentiles, and active connection count where applicable.
- [ ] Invalid `window` values return HTTP 400 with the existing API error envelope.
- [ ] Tests cover empty state.
- [ ] Tests cover populated state.
- [ ] Tests cover invalid `window`.
- [ ] `cargo fmt --check` passes.
- [ ] `cargo test --workspace` passes.

## Out of Scope

- Historical statistics queries.
- Filtering statistics by `protocol`, `database_type`, `database`, or `user`.
- WebSocket statistics stream.
- Persistent statistics storage.
- Top SQL fingerprints, top users, or top databases.
- Frontend dashboard integration.
