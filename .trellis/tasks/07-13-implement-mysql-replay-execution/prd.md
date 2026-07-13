# Implement Guarded MySQL Replay Execution

## Goal

Execute a selected captured or user-supplied MySQL-compatible SQL statement against an explicitly selected configured target, with mutation confirmation and bounded failure behavior.

## Requirements

- Add an execution endpoint separate from preview.
- Require an explicit target/backend selection; never infer a production destination from an event alone.
- Require confirmation for mutations when configured.
- Use redacted/captured event data safely and never expose stored secrets in errors or logs.
- Apply connect and execution timeouts and return protocol-neutral result/error summaries.
- Keep replay disabled when `replay.enabled = false`.

## Acceptance Criteria

- Read-only replay succeeds against a test MySQL-compatible backend.
- Mutation replay is rejected without explicit confirmation when required.
- Disabled replay is rejected consistently.
- Missing event, invalid SQL, backend failure, and timeout return typed API errors.
- No replay execution occurs against an implicit or unconfigured target.

## Out Of Scope

- PostgreSQL or ClickHouse replay.
- Transaction orchestration, rollback guarantees, or SQL rewriting.
