# SQL Lens AI Agent Guide

This document is the operating guide for AI coding agents working on SQL Lens.

SQL Lens is a developer-first, multi-protocol SQL Debug Proxy. The first implementation target is the MySQL-compatible protocol family, but the architecture must preserve extension points for PostgreSQL, SQLite integration, ClickHouse, and other SQL execution surfaces.

## Prime Directive

Do not turn SQL Lens into a general database middleware.

SQL Lens observes, captures, explains, and replays SQL for debugging. It does not own database traffic policy, sharding, high availability, read/write splitting, data synchronization, or SQL rewrite in the open source core.

## Current Project State

At bootstrap, the repository is documentation-first. Business code may not exist yet. When adding implementation, create the smallest module that satisfies the current milestone and keep it aligned with the documented architecture.

## Recommended Directory Structure

```text
sql-lens/
  crates/
    sql-lens-core/
    sql-lens-config/
    sql-lens-proxy/
    sql-lens-protocol/
    sql-lens-protocol-mysql/
    sql-lens-storage/
    sql-lens-api/
    sql-lens-plugin/
    sql-lens-app/
  web/
    src/
      app/
      components/
      features/
      lib/
      types/
      styles/
  docs/
  examples/
  tests/
```

## Rust Crate Responsibilities

### `sql-lens-core`

Owns protocol-neutral domain types:

- SQL events.
- Connections.
- Parameters.
- Timing.
- Result summaries.
- Error summaries.
- Redaction state.
- Protocol metadata container.

Do not depend on protocol-specific crates from core.

### `sql-lens-config`

Owns runtime configuration contracts:

- Startup configuration structs.
- Configuration enums.
- Default values.
- Serde-compatible configuration shape.

Do not load files, read environment variables, validate runtime constraints, or start services here.

### `sql-lens-proxy`

Owns network forwarding:

- TCP listener.
- Backend dialing.
- Session lifecycle.
- Bidirectional forwarding.
- Shutdown.
- Backpressure coordination.

Do not put SQL parsing or UI schemas here.

### `sql-lens-protocol`

Owns protocol adapter traits and registry.

Protocol adapters emit shared capture events and attach protocol-specific metadata.

### `sql-lens-protocol-mysql`

Owns MySQL-compatible protocol parsing:

- Packet framing.
- Handshake observation.
- Authentication state.
- Commands.
- Prepared statement lifecycle.
- Parameter decoding.
- Error packet mapping.

Do not leak MySQL-only assumptions into shared API types.

### `sql-lens-storage`

Owns:

- Ring buffer.
- SQLite.
- Future DuckDB.
- Retention.
- Query filters.
- Statistics helpers.

Storage receives already-redacted events by default.

### `sql-lens-api`

Owns:

- REST handlers.
- WebSocket handlers.
- API error mapping.
- OpenAPI schema generation.

Do not parse protocol packets here.

### `sql-lens-plugin`

Owns:

- Hook traits.
- Exporter traits.
- Plugin lifecycle.
- Plugin safety boundaries.

Plugins must not block packet forwarding.

### `sql-lens-app`

Owns composition:

- CLI.
- Config load.
- Logging setup.
- Runtime startup.
- Graceful shutdown.

## Frontend Responsibilities

Use React, TypeScript, TailwindCSS, shadcn/ui, TanStack Query, Monaco Editor, and ECharts.

Recommended feature directories:

- `features/dashboard`
- `features/sql-events`
- `features/connections`
- `features/statistics`
- `features/replay`
- `features/settings`

Rules:

- TanStack Query owns server state.
- URL state owns filters that should survive reloads.
- Local component state owns temporary UI state.
- Avoid `any`.
- Treat SQL text and database errors as untrusted text.
- Do not render SQL as HTML.

## Public Contracts

Be careful when modifying:

- `SqlEvent`.
- `SqlParameter`.
- `ConnectionInfo`.
- REST response schemas.
- WebSocket message types.
- Plugin hook payloads.
- Storage query filters.
- Configuration keys.

These contracts are shared across backend, frontend, docs, and future plugins.

## Modules That Can Be Developed Independently

Usually independent:

- Ring buffer storage.
- Config parsing.
- REST health endpoint.
- Web UI layout.
- Dashboard charts using mocked API.
- Documentation.
- Protocol fixture tests.
- Redaction rule engine.

Need coordination:

- Capture event schema changes.
- API response changes.
- WebSocket message changes.
- Protocol metadata shape.
- Replay behavior.
- Security and redaction behavior.
- Plugin hook signatures.

## Forbidden Patterns

- Do not log passwords or authentication packet payloads.
- Do not persist unredacted sensitive parameters when redaction is enabled.
- Do not block packet forwarding on UI, storage, plugins, or exporters.
- Do not add MySQL-specific fields to protocol-neutral structs unless they are inside metadata.
- Do not implement SQL rewrite unless a future task explicitly changes scope.
- Do not add broad abstractions without a second concrete use.
- Do not create package manifests or source code in documentation-only tasks.
- Do not remove the Trellis block in this file.
- Do not commit or push unless explicitly requested.

## Naming

Use:

- Product: `SQL Lens`.
- Protocol family: `MySQL-compatible`.
- Adapter names: `mysql`, `postgresql`, `clickhouse`, `sqlite`.
- Event names: snake_case in JSON.
- Rust types: PascalCase.
- Rust modules: snake_case.
- React components: PascalCase.
- Hooks: `useSomething`.

Avoid:

- Reintroducing the old MySQL-only product name from early drafts.
- Naming shared types as if every event is MySQL-only.

## Testing Rules

Before merging protocol work:

- Unit tests for parser logic.
- Golden packet fixture tests.
- Integration test with a live database when practical.

Before merging storage work:

- Capacity tests.
- Eviction tests.
- Filter tests.
- Retention tests.

Before merging API work:

- Schema tests.
- Error response tests.
- Pagination tests.

Before merging UI work:

- Component tests for behavior.
- Playwright smoke test for major flows when available.

Before merging security-sensitive work:

- Redaction tests.
- XSS tests for rendered SQL and errors.
- CSRF tests for mutating endpoints.
- Replay confirmation tests.

## AI Workflow

1. Read the active task artifacts.
2. Read relevant docs before editing.
3. Search before changing shared names or schemas.
4. Keep changes scoped.
5. Update docs when behavior changes.
6. Run the narrowest useful validation first.
7. Run broader validation before final report.
8. Report what changed and what was not tested.

## KISS, YAGNI, DRY

KISS:

- Prefer direct code and explicit state machines.
- Keep hot path logic simple.

YAGNI:

- Do not build generic multi-protocol machinery beyond the adapter boundary until a second protocol needs it.
- Do not build enterprise auth before local auth is stable.

DRY:

- Shared event models belong in core.
- Shared API schemas should be generated or reused by frontend types when practical.
- Avoid duplicating redaction logic across storage, API, and exporters.

## Trellis

<!-- TRELLIS:START -->
# Trellis Instructions

These instructions are for AI assistants working in this project.

This project is managed by Trellis. The working knowledge you need lives under `.trellis/`:

- `.trellis/workflow.md` — development phases, when to create tasks, skill routing
- `.trellis/spec/` — package- and layer-scoped coding guidelines (read before writing code in a given layer)
- `.trellis/workspace/` — per-developer journals and session traces
- `.trellis/tasks/` — active and archived tasks (PRDs, research, jsonl context)

If a Trellis command is available on your platform (e.g. `/trellis:finish-work`, `/trellis:continue`), prefer it over manual steps. Not every platform exposes every command.

If you're using Codex or another agent-capable tool, additional project-scoped helpers may live in:
- `.agents/skills/` — reusable Trellis skills
- `.codex/agents/` — optional custom subagents

Managed by Trellis. Edits outside this block are preserved; edits inside may be overwritten by a future `trellis update`.

<!-- TRELLIS:END -->
