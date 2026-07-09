# Issue 099: Add OpenAPI generation

## Goal

Generate the SQL Lens v1 OpenAPI document from backend-owned API contracts so frontend work and release packaging can consume a stable REST API description.

## Background

- `ISSUES.md` requires `docs/openapi/sql-lens.v1.yaml`.
- `API.md` says every release should publish an OpenAPI document and REST endpoints must be represented.
- The repository currently has no `docs/openapi/` output and no OpenAPI generation command.
- The repository currently has no `.github` workflow files, so this task should add a stale-output test that future CI can run instead of creating CI from scratch.
- The frontend agent can use the generated OpenAPI file for API alignment once it exists.

## Requirements

- Generate `docs/openapi/sql-lens.v1.yaml` from Rust backend API contract code.
- Represent the current REST endpoints documented in `API.md`, including:
  - `GET /api/v1/health`
  - `GET /api/v1/sql-events`
  - `GET /api/v1/sql-events/{id}`
  - `GET /api/v1/sql-events/export`
  - `GET /api/v1/connections`
  - `GET /api/v1/connections/{id}`
  - `GET /api/v1/statistics`
  - `GET /api/v1/protocols`
  - `POST /api/v1/replay/preview`
- Include shared success and error schemas used by those REST endpoints.
- Keep WebSocket behavior out of the OpenAPI paths. WebSocket message payload schemas may be included as components only if low-risk and directly supported by existing DTOs.
- Add a deterministic generation command that can refresh the YAML file.
- Add an automated stale-output check that fails when generated OpenAPI differs from the committed YAML.
- Do not add frontend code, OpenAPI UI hosting, auth behavior, replay execute, or GitHub Actions workflows in this task.

## Acceptance Criteria

- [x] `docs/openapi/sql-lens.v1.yaml` exists.
- [x] A documented command can regenerate the OpenAPI YAML.
- [x] Current REST endpoints from `API.md` are represented in the generated YAML.
- [x] Shared API error envelope schema is represented.
- [x] Stale generated output is detected by a Rust test or equivalent repo-local check.
- [x] `rtk cargo fmt --check` passes.
- [x] `rtk cargo test -p sql-lens-api` passes.
- [x] `rtk cargo test --workspace` passes.
- [x] `rtk cargo clippy --workspace --all-targets -- -D warnings` passes.

## Out Of Scope

- Serving Swagger UI or Redoc from the app.
- Generating frontend TypeScript clients.
- Adding GitHub Actions CI workflows.
- Adding or changing REST endpoint behavior.
- Documenting protocol-level MySQL wire traffic.
- Replay execute or mutating endpoint security.

## Open Questions

None blocking. The initial implementation should prefer a code-first Rust generator and a staleness test over hand-maintained YAML.
