# Capture COM_QUERY timing plan

## Checklist

- [x] Read backend specs before implementation.
- [x] Add MySQL-local observation clock abstraction.
- [x] Store connection context needed for event construction in MySQL state.
- [x] Add pending query timing state.
- [x] Start pending timing on valid `COM_QUERY` after authentication.
- [x] Detect backend OK terminal response for pending query.
- [x] Detect backend ERR terminal response for pending query.
- [x] Build normalized `SqlEvent` from pending query and connection context.
- [x] Emit one event on OK finalization.
- [x] Emit one event on ERR finalization.
- [x] Keep unsupported backend responses non-fatal and pending.
- [x] Ensure `ProtocolObservation.events_emitted` is accurate.
- [x] Add success timing tests.
- [x] Add error timing tests.
- [x] Add no-pending and unsupported-response tests.
- [x] Update backend spec with COM_QUERY timing contract.
- [x] Run `rtk cargo fmt --check`.
- [x] Run `rtk cargo test -p sql-lens-protocol-mysql`.
- [x] Run `rtk cargo test --workspace`.
- [x] Run `rtk cargo clippy --workspace --all-targets -- -D warnings`.

## Validation Commands

```bash
rtk cargo fmt --check
rtk cargo test -p sql-lens-protocol-mysql
rtk cargo test --workspace
rtk cargo clippy --workspace --all-targets -- -D warnings
```

## Risk Notes

- Do not add `time`, `chrono`, or `uuid`.
- Do not import capture pipeline, storage, API, proxy, or app crates.
- Do not parse detailed OK/ERR summaries in this task.
- Do not emit multiple events for one pending query.
- Do not log raw SQL text.

## Review Gate

Before implementation starts, confirm:

- The adapter may emit `SqlEvent` through the existing protocol `CaptureEventEmitter`.
- Minimal OK/ERR detection by first payload byte is acceptable for Issue 044.
- Event IDs use deterministic per-connection counters until a later global ID strategy exists.
