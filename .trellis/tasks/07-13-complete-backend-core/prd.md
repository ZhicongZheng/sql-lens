# Complete Backend Core Runtime Capabilities

## Goal

Close the highest-priority backend runtime gaps identified after the MySQL-compatible MVP: connection governance, configured redaction, retention correctness, guarded replay execution, protocol runtime composition, and plugin dispatch.

## Task Map

This parent task is planning-only and is implemented through these ordered child tasks:

1. `07-13-proxy-connection-governance`
2. `07-13-wire-runtime-redaction-config`
3. `07-13-complete-runtime-retention`
4. `07-13-implement-mysql-replay-execution`
5. `07-13-integrate-protocol-registry-runtime`
6. `07-13-implement-plugin-runtime`

## Constraints

- Preserve SQL Lens as a local developer debugging proxy, not a general database middleware.
- Keep packet forwarding independent from storage, UI, plugins, and exporters.
- Keep protocol-neutral contracts in shared crates; MySQL-specific behavior stays in the MySQL adapter.
- Do not introduce PostgreSQL, ClickHouse, DuckDB, or TLS termination in this parent task; those remain separate expansion work.
- Do not commit or push as part of these tasks.

## Parent Acceptance Criteria

- Each child task has its own reviewed implementation and quality-check results.
- The final runtime honors the configuration fields that the child tasks claim to support.
- Workspace tests, formatting, and Clippy pass after the ordered child tasks are integrated.
- Existing MySQL proxy, capture, API, SQLite, and WebSocket behavior remains compatible.
