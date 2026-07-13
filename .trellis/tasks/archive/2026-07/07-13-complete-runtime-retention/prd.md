# Complete Runtime Retention Policy Enforcement

## Goal

Make configured retention predictable and complete for the runtime-owned Ring Buffer and SQLite stores.

## Confirmed Gaps

- Ring Buffer age retention is not applied: `crates/sql-lens-app/src/retention.rs:65-73`.
- `max_bytes` is rejected at enforcement time.
- Retention config is cloned at startup, so changes cannot affect the next cycle.
- The current config has only global retention fields; per-table/per-query overrides do not exist.

## Requirements

- Enforce max age and max event count for both runtime storage backends where supported.
- Use one validated timestamp representation for capture, storage, filtering, and cutoff calculations.
- Define and validate unsupported `max_bytes` behavior rather than logging a recurring runtime failure.
- Read retention settings from a runtime-owned configuration source on each cycle, or explicitly narrow the contract to restart-only and update docs/tests.
- Continue enforcement after one backend/table operation fails and emit before/after counts.
- Keep cleanup off the packet-forwarding Tokio worker and avoid unbounded deletion transactions.

## Acceptance Criteria

- Old events are deleted while newer events are preserved in Ring Buffer and SQLite integration tests.
- Event-count cleanup preserves newest rows and removes related SQLite parameter rows.
- Invalid or unsupported retention settings fail validation/startup clearly.
- A changed retention setting is observed on the next enforcement cycle if dynamic reload remains in scope.
- A cleanup failure is logged and does not terminate the runtime.
