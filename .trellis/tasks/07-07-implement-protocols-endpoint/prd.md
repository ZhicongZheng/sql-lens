# Implement protocols endpoint

## Goal

Implement Issue 032: add `GET /api/v1/protocols` so clients can discover SQL Lens protocol families without hard-coding product roadmap knowledge.

## Requirements

- Add a read-only REST endpoint at `GET /api/v1/protocols`.
- Return a protocol-neutral response with an `items` array.
- Include `mysql` with status `supported` and compatible database targets `mysql`, `starrocks`, `tidb`, and `doris`.
- Include planned protocol families so the UI can present roadmap-aware affordances without MySQL-only assumptions.
- Keep the first implementation static; do not wire runtime adapter discovery until protocol registry composition exists in the app.
- Do not add protocol-specific fields outside the per-item `databases` list.

## Acceptance Criteria

- [x] `GET /api/v1/protocols` returns HTTP 200.
- [x] The response includes `mysql` as `supported`.
- [x] The MySQL-compatible item lists `mysql`, `starrocks`, `tidb`, and `doris`.
- [x] Planned protocols are marked `planned`.
- [x] Response structs are protocol-neutral and serializable.
- [x] Tests cover the endpoint response.
- [x] `cargo fmt --check` passes.
- [x] `cargo test --workspace` passes.

## Out of Scope

- Dynamic adapter registry inspection.
- Feature flags for enabling or disabling protocol adapters.
- Protocol health checks.
- Frontend integration.
