# Add live statistics counters

## Goal

Issue 025: add lightweight live statistics counters for dashboard metrics.

## User Value

Developers need a fast live summary of captured SQL traffic before the REST statistics endpoint and frontend dashboard exist.

## Confirmed Facts

- Issue 018 is complete: `sql-lens-capture` provides a bounded capture event channel and dropped-event counter.
- Issue 021 is complete: `sql-lens-storage` provides the default in-memory ring buffer storage.
- Current `SqlEvent` contains `status`, `duration`, `fingerprint`, `database`, `user`, and `connection_id`.
- Current `CaptureStatus` variants are `Ok`, `Slow`, `Error`, and `Unknown`.
- `STORAGE.md` says live dashboard statistics should use incremental counters, while historical ranges should use storage queries.
- `PRD.md` lists statistics metrics such as QPS, error rate, slow query count, latency percentiles, top fingerprints, top databases, top users, and active connections.
- `ISSUES.md` Issue 025 requires QPS, errors, slow SQL, latency buckets, active connections, and tests.
- There is no dedicated `sql-lens-statistics` crate in the current workspace.
- `sql-lens-storage` is documented as owning statistics helpers in the current crate responsibility docs.
- Decision: active connections are updated through explicit connection lifecycle methods, not inferred from SQL events.
- Decision: the first implementation lives in `sql-lens-storage` as statistics helpers rather than adding a new crate.
- Decision: the first QPS metric uses a fixed 60-second live window. Future API work can layer named windows such as 1m, 5m, and 15m.
- Decision: latency buckets use fixed millisecond upper bounds in this task; percentile calculation is left to a later task.

## Requirements

- Add a lightweight in-memory live statistics component.
- Counters must update from `SqlEvent` values without blocking capture forwarding.
- Track total query events.
- Track error SQL count from `CaptureStatus::Error`.
- Track slow SQL count from `CaptureStatus::Slow`.
- Track latency buckets from `SqlEvent.duration`.
- Track QPS over a fixed 60-second live window based on event ingestion time.
- Track active connections from explicit open/close lifecycle calls.
- Expose a snapshot type for future API/dashboard consumption.
- Add tests for counter updates.

## Out Of Scope

- REST statistics endpoint.
- WebSocket statistics stream.
- Frontend dashboard.
- Persistent statistics.
- Historical statistics from SQLite/DuckDB.
- Percentile calculation unless explicitly included in the first counter contract.
- Top fingerprint/database/user rankings unless explicitly included in this issue.
- Multi-window statistics beyond the default 60-second live window.
- Connection lifecycle persistence.

## Acceptance Criteria

- [x] Counter accepts `SqlEvent` updates.
- [x] Snapshot reports total events.
- [x] Snapshot reports error count.
- [x] Snapshot reports slow count.
- [x] Snapshot reports latency buckets.
- [x] Snapshot reports QPS over the fixed 60-second live window.
- [x] Active connections update through explicit open/close methods.
- [x] Tests cover counter updates for ok, slow, and error events.
- [x] Tests cover latency bucket assignment.
- [x] Tests cover QPS/window behavior.
- [x] No async runtime or API dependency is added.
- [x] `cargo fmt --check` passes.
- [x] `cargo check --workspace` passes.
- [x] `cargo test --workspace` passes.
- [x] `cargo clippy --workspace --all-targets -- -D warnings` passes.

## Open Questions

None blocking.
