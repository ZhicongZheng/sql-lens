# Add live statistics counters design

## Boundary

Implement in `crates/sql-lens-storage`.

No new dependencies.

This task adds in-memory statistics helpers only. It does not add REST handlers, WebSocket streams, frontend widgets, persistent statistics, historical queries, percentile calculation, top-N rankings, or connection lifecycle persistence.

## Public API

Planned public types:

```rust
pub struct LiveStatistics;

impl LiveStatistics {
    pub fn new() -> Self;
    pub fn record_sql_event(&mut self, event: &SqlEvent);
    pub fn record_sql_event_at(&mut self, event: &SqlEvent, recorded_at: std::time::Instant);
    pub fn record_connection_opened(&mut self, connection_id: ConnectionId);
    pub fn record_connection_closed(&mut self, connection_id: &ConnectionId);
    pub fn snapshot(&mut self) -> LiveStatisticsSnapshot;
    pub fn snapshot_at(&mut self, now: std::time::Instant) -> LiveStatisticsSnapshot;
}

pub struct LiveStatisticsSnapshot {
    pub total_events: u64,
    pub error_events: u64,
    pub slow_events: u64,
    pub qps_window_secs: u64,
    pub qps: f64,
    pub latency_buckets: Vec<LatencyBucketCount>,
    pub active_connections: usize,
}

pub struct LatencyBucketCount {
    pub upper_bound: Option<DurationMillis>,
    pub count: u64,
}
```

`LiveStatistics::default()` should be equivalent to `LiveStatistics::new()`.

## SQL Event Counters

`record_sql_event` updates:

- `total_events` for every `SqlEvent`.
- `error_events` when `event.status == CaptureStatus::Error`.
- `slow_events` when `event.status == CaptureStatus::Slow`.
- one latency bucket based on `event.duration`.
- recent event timestamps for QPS.

`CaptureStatus::Unknown` increments only `total_events` and latency buckets.

## QPS Semantics

The first implementation uses a fixed 60-second live window.

QPS is calculated as:

```text
events recorded in the last 60 seconds / 60.0
```

Event time for live QPS is ingestion time, represented by `std::time::Instant`, not the `SqlEvent.timestamp` string. This avoids timestamp parsing and keeps tests deterministic through `record_sql_event_at` and `snapshot_at`.

Recent event timestamps are pruned on record and snapshot calls so memory stays bounded by recent traffic.

## Latency Buckets

Use fixed millisecond upper bounds:

```text
<=1ms, <=5ms, <=10ms, <=50ms, <=100ms, <=500ms, <=1000ms, <=5000ms, >5000ms
```

Represent the overflow bucket as `upper_bound = None`.

This task intentionally does not calculate p50/p95/p99. Future API/statistics work can derive percentiles from richer histograms or storage queries.

## Active Connections

Active connections are not inferred from SQL events.

`record_connection_opened(ConnectionId)` inserts the connection ID into an internal set.

`record_connection_closed(&ConnectionId)` removes the connection ID if present.

Repeated opens for the same connection are idempotent. Closing a missing connection is a no-op.

`LiveStatisticsSnapshot.active_connections` is the current set length.

## Ownership And Dependencies

The first implementation belongs in `sql-lens-storage` because current project docs assign storage helpers and statistics helpers to that crate.

Allowed dependencies remain:

```toml
sql-lens-core = { path = "../sql-lens-core" }
```

Use standard library `std::time::{Duration, Instant}` and `std::collections::{HashSet, VecDeque}`.

Do not add `tokio`, `serde`, `time`, `uuid`, `parking_lot`, metrics libraries, API crates, capture crates, or proxy crates for this task.

## Tests

Tests should construct synthetic `SqlEvent` values and use deterministic `Instant` values.

Required coverage:

- OK event increments total and latency only.
- Error event increments total and error count.
- Slow event increments total and slow count.
- Latency bucket assignment across at least low, middle, and overflow buckets.
- QPS window includes events inside the 60-second window and excludes older events.
- Active connection open/close updates set length and is idempotent for repeated close.
