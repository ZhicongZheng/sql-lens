# SQL Lens Documentation Design

## Objective

Produce the foundational documentation set for SQL Lens as if this repository were ready to publish as a serious open source project, while keeping the actual codebase empty of business implementation.

## Documentation Architecture

The documentation should use root-level files for first-visit discoverability. The root docs are the canonical contract for the initial project direction.

```text
README.md         Entry point for users and contributors
PRD.md            Product requirements and scope
ARCHITECTURE.md   System design and runtime model
PROTOCOL.md       Database protocol design
API.md            REST and WebSocket API contract
CONFIG.md         Configuration model
STORAGE.md        Capture storage model
SECURITY.md       Security and privacy design
PLUGIN.md         Extension model
ROADMAP.md        Version roadmap
MILESTONE.md      PR-sized work breakdown to v1.0
ISSUES.md         GitHub issue backlog
TESTING.md        Test strategy
BENCHMARK.md      Performance goals and benchmark method
UI.md             Web UI product design
CONTRIBUTING.md   Contribution workflow
AGENTS.md         AI coding agent operating guide
```

## Naming and Positioning

Use "SQL Lens" as the canonical name everywhere.

The source `SQL_Lens_PRD.md` uses "SQL Lens" as the canonical product name. Documentation should treat MySQL-compatible databases as the first protocol family, not the product identity or final protocol boundary.

## Language

Project documentation should be written in English. This matches `.trellis/spec/backend/index.md` and `.trellis/spec/frontend/index.md`.

Short examples may include SQL, JSON, TOML, shell commands, and ASCII architecture diagrams.

## Product Boundary

SQL Lens should be documented as:

- Developer-first.
- Transparent by database address change.
- Focused on debug, observability, audit, and analysis.
- Protocol-aware enough to reconstruct SQL.
- Non-invasive to application code.

SQL Lens should not be documented as:

- A production database middleware.
- A high availability proxy.
- A sharding or read/write splitting system.
- A SQL rewrite engine.
- A governance or policy enforcement platform.

## Technical Direction

Backend:

- Rust-first.
- Tokio async runtime.
- Separate crates for protocol, proxy, capture model, storage, API, config, plugin, and app binary.
- Keep protocol adapters independent from the shared capture model so PostgreSQL, SQLite integration, ClickHouse, and other future SQL surfaces can be added later.
- Ring buffer as default storage.
- SQLite as optional persistent storage.
- DuckDB as future analytical storage.

Frontend:

- React.
- TypeScript.
- TailwindCSS.
- shadcn/ui.
- TanStack Query.
- Monaco Editor.
- ECharts.

Go:

- Optional package layout should be documented only as a fallback or alternative contributor path.
- Rust remains the preferred implementation direction.

## Cross-Document Consistency Rules

- Feature names must be consistent across docs.
- Roadmap versions must match across `README.md`, `ROADMAP.md`, `MILESTONE.md`, and `ISSUES.md`.
- API resource names must match between `API.md`, `ARCHITECTURE.md`, `UI.md`, and `AGENTS.md`.
- Storage event fields must match between `STORAGE.md`, `API.md`, `PROTOCOL.md`, and `UI.md`.
- Plugin hook names must match between `PLUGIN.md`, `ARCHITECTURE.md`, and `AGENTS.md`.
- Security-sensitive behavior such as redaction must be described consistently in `SECURITY.md`, `CONFIG.md`, `STORAGE.md`, and `AGENTS.md`.

## Document Scope

`README.md` should be concise but complete enough for a first-time GitHub visitor.

Deep technical docs should avoid vague marketing language and provide implementation-ready structure:

- States.
- Data models.
- Lifecycles.
- Error handling.
- Boundaries.
- Trade-offs.
- Non-goals.

`ISSUES.md` should be directly copyable into GitHub issues. Each issue must include:

- Title.
- Description.
- Acceptance criteria.
- Labels.
- Priority.
- Difficulty.
- Estimated time.
- Dependencies.

## Validation Strategy

Because this task is documentation-only:

- Verify every required root documentation file exists.
- Check that `ISSUES.md` includes at least 100 issue entries.
- Check for stale "MySQL Lens" naming in generated documentation, except where intentionally referencing the old seed name.
- Check that no business source files or build manifests are created accidentally.
- Use lightweight shell validation rather than compile or unit test commands.

## Risks

- The requested documentation volume is large, so consistency drift is the main risk.
- Protocol details can become over-specific before implementation begins. Docs should clearly mark unsupported or future behavior.
- Security docs must avoid implying production safety before the proxy has real hardening.
- The issue backlog can become too broad. Each issue should remain PR-sized and independently reviewable.

## Rollback Shape

If generated docs need rollback, remove or revise only files created by this task. Do not modify Trellis instructions outside the task artifacts unless explicitly requested.
