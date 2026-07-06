# Track connection lifecycle implementation plan

## Steps

1. Add `sql-lens-core` as a dependency of `sql-lens-proxy`.
2. Add lifecycle ID generation and lifecycle record types to `crates/sql-lens-proxy/src/lib.rs`.
3. Add transition methods for accepted, backend connected, forwarding closed, backend dial failed, and forwarding failed.
4. Add lightweight unit tests for ID generation, normal close, and backend dial failure.
5. Run workspace validation.

## Constraints

- Keep implementation in `sql-lens-proxy`.
- Keep records protocol-neutral.
- Do not add UUID, time, chrono, storage, API, app runtime, protocol parsing, or capture pipeline dependencies.
- Do not wire lifecycle tracking into an async runtime loop yet; session orchestration comes later.

## Acceptance mapping

- Connection ID generation: `ConnectionLifecycleIdGenerator`.
- Created/accepted state: `ConnectionLifecycleRecord::accepted`.
- Backend connected state: `mark_backend_connected`.
- Normal close: `mark_forwarding_closed`.
- Backend failure: `mark_backend_dial_failed`.
- Byte counters: copied from `ForwardingSummary`.
