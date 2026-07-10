# Add retention policy enforcement

## Goal

Enforce max age, max events, and max bytes retention policies on ring-buffer and SQLite storage backends to prevent unbounded memory/disk growth while preserving the most recent events.

## Background

This task depends on:
- Issue 021: (ring buffer implementation)
- Issue 087: (SQLite event storage implementation)

The storage layer needs active cleanup mechanisms that respect configured retention limits.

## Requirements

- Ring buffer must enforce `max_events` limit by dropping oldest events when capacity exceeded
- SQLite must support time-based cleanup (max age) and count-based cleanup (max events)
- Max bytes retention remains explicitly unsupported pending separate design
- Cleanup operations must not block capture writes (enforcement strategy decided at app runtime layer)
- Tests must verify cleanup behavior for both storage types
- Retention enforcement applies global configuration only (per-table/per-query overrides deferred to future work)
- Storage layer provides synchronous cleanup methods; async scheduling is app runtime responsibility

## Acceptance Criteria

- [ ] Ring buffer respects `max_events` configuration
- [ ] SQLite supports age-based cleanup (delete events older than configured age)
- [ ] SQLite supports event-count cleanup (delete oldest events when exceeding max)
- [ ] Tests cover cleanup behavior for ring-buffer and SQLite
- [ ] Max bytes retention explicitly documented as unsupported

## Technical Notes

**Confirmed from codebase inspection:**
- Ring buffer (`RingBufferStore`) implements `enforce_max_events(&mut self, max_events: NonZeroUsize)` — drops oldest events when exceeding capacity (ring_buffer.rs:68-80)
- SQLite (`SqliteEventStore`) implements:
  - `delete_events_older_than(&mut self, cutoff: &Timestamp)` — age-based cleanup with transaction (sqlite_event_store.rs:307-333)
  - `enforce_max_events(&mut self, max_events: NonZeroUsize)` — event-count cleanup, deletes oldest by timestamp (sqlite_event_store.rs:335-364)
- `RetentionConfig` exists in `crates/sql-lens-config/src/model.rs` with `max_age: String` (default "24h"), `max_events: u64` (default 100_000), `max_bytes: Option<u64>`, `drop_policy: RetentionDropPolicy`
- No retention enforcement is currently wired into `sql-lens-app` runtime
- `max_bytes` retention has no implementation (consistent with AC: explicitly unsupported)
- Unit tests exist for storage layer cleanup: `ring_buffer_retention_*` (ring_buffer.rs:1106+), `sqlite_retention_*` (sqlite_event_store.rs:1192+)

**Scope clarification:**
- Issue 089 confirms storage layer cleanup capability and test coverage
- App runtime integration of `RetentionConfig` (calling cleanup methods during capture) is deferred to Issue 117
- Per-table/per-query retention overrides are out of scope for Issue 089

**Dependencies:**
- Issue 021: Ring buffer implementation (provides `enforce_max_events`)
- Issue 087: SQLite event storage (provides `delete_events_older_than` and `enforce_max_events`)

## Open Questions

None at this time — pending codebase inspection.

## Notes

- This task primarily confirms existing storage layer cleanup capabilities and ensures test coverage
- Core implementation (`enforce_max_events`, `delete_events_older_than`) already exists in `sql-lens-storage`
- App runtime integration deferred to Issue 117
- Per-table overrides deferred to future work
- Task complexity: Medium (6h) — mostly verification, documentation, and potential test gap filling
