# Implement storage filters

## Goal

Issue 024: add storage-level filters for retained SQL events so the SQL timeline and future `GET /api/v1/sql-events` endpoint can query the ring buffer by common debugging dimensions.

## User Value

Developers need to narrow the SQL timeline quickly by protocol, database, user, status, duration, SQL text, and time window without waiting for SQLite or API work.

## Confirmed Facts

- Issue 023 is complete and `RingBufferStore::query_timeline` now returns retained events newest-first with cursor pagination.
- `ISSUES.md` requires filters for protocol, database type, database, user, status, duration, text, and time range.
- `API.md` lists future SQL event query parameters including `limit`, `cursor`, `protocol`, `database_type`, `database`, `user`, `client_addr`, `status`, `min_duration_ms`, `max_duration_ms`, `q`, `fingerprint`, `from`, and `to`.
- `PRD.md` lists search filters for SQL text, fingerprint, protocol, database type, database, user, client IP, status, duration range, and time range.
- `Timestamp` is currently a string newtype with ordering but no time parsing dependency.
- The storage crate currently has no dependencies beyond `sql-lens-core`.
- Decision: storage filters are strongly typed. Unknown HTTP query parameters will be rejected later by the API layer before storage is called.
- Decision: storage returns typed errors only for invalid supported filter combinations, such as reversed duration or timestamp ranges.

## Requirements

- Add filter support to ring buffer timeline queries.
- Supported Issue 024 filters:
  - protocol
  - database type
  - database
  - user
  - status
  - minimum duration
  - maximum duration
  - SQL text
  - start timestamp
  - end timestamp
- Filters must be combinable with existing limit and cursor pagination.
- Filtered timeline results must keep newest-first order.
- Cursor semantics must remain stable across newer appends.
- Text filtering must only inspect stored event text fields; it must not parse SQL.
- Time filtering must not add a time parsing dependency in this task.
- Invalid supported filter combinations must return clear typed errors instead of being ignored.
- Existing append, lookup, snapshot, stats, and unfiltered timeline behavior must remain unchanged.

## Out Of Scope

- API query parameter parsing.
- WebSocket subscription filters.
- SQLite or DuckDB filter implementation.
- Secondary indexes.
- SQL parsing or full-text search.
- Timestamp parsing with a new time crate.
- Fingerprint and client address filters unless explicitly pulled into this task.
- Unknown HTTP query parameter handling. That belongs to the future API list endpoint.

## Acceptance Criteria

- [x] Filters can be combined.
- [x] At least five filter combinations are covered by tests.
- [x] Invalid supported filter combinations return clear typed errors.
- [x] Filtered results preserve newest-first ordering.
- [x] Filtered results preserve cursor pagination without duplicates.
- [x] Existing unfiltered timeline tests still pass.
- [x] Existing append/get/snapshot/stats tests still pass.
- [x] No new dependency is added.
- [x] `cargo fmt --check` passes.
- [x] `cargo check --workspace` passes.
- [x] `cargo test --workspace` passes.
- [x] `cargo clippy --workspace --all-targets -- -D warnings` passes.

## Open Questions

None blocking.
