# Implement statistics endpoint design

## Boundary

Implement a read-only API endpoint in `crates/sql-lens-api` backed by live in-memory statistics from `crates/sql-lens-storage`.

This task may extend `LiveStatistics` with bounded recent latency samples because the endpoint contract requires p50/p95/p99. It must not introduce historical analytics, persistent storage, background jobs, protocol parsing, frontend code, or WebSocket streaming.

## API Contract

Path:

```http
GET /api/v1/statistics
```

Query parameters:

- `window`: optional. Defaults to `1m`.

Supported values for this first live endpoint:

- `1m`
- `60s`

All other values return `400 Bad Request`.

The broader `API.md` lists additional filters (`protocol`, `database_type`, `database`, `user`), but those are out of scope until storage-backed historical statistics exist.

Response:

```json
{
  "window": "1m",
  "qps": 0.0,
  "error_rate": 0.0,
  "slow_count": 0,
  "latency_ms": {
    "p50": 0.0,
    "p95": 0.0,
    "p99": 0.0
  },
  "active_connections": 0
}
```

## Statistics Semantics

The endpoint returns a live snapshot from `LiveStatistics`.

- `qps`: existing fixed 60-second ingestion-time QPS.
- `error_rate`: `error_events / total_events`; empty state returns `0.0`.
- `slow_count`: `slow_events`.
- `active_connections`: explicit live connection lifecycle count.
- `latency_ms`: percentiles over recent live latency samples retained for the same 60-second window as QPS.

Percentiles should be exact over the bounded live sample set, not approximated from latency buckets. This keeps the API honest and leaves bucket counters available for coarse charts.

## Storage Changes

Extend `LiveStatistics` with a `VecDeque<LatencySample>`:

```rust
struct LatencySample {
    recorded_at: Instant,
    duration: DurationMillis,
}
```

On `record_sql_event_at`:

- prune expired recent timestamps and latency samples,
- push the event timestamp into the QPS deque,
- push the event duration into the latency sample deque.

On `snapshot_at`:

- prune expired recent timestamps and latency samples,
- calculate p50/p95/p99 from copied and sorted recent sample durations.

Expose the percentile values in `LiveStatisticsSnapshot` as a small public struct:

```rust
pub struct LatencyPercentiles {
    pub p50: f64,
    pub p95: f64,
    pub p99: f64,
}
```

Keep existing snapshot fields and existing tests compatible.

## API State

Add a shared `LiveStatistics` handle to `ApiState`:

```rust
Arc<RwLock<LiveStatistics>>
```

Keep existing constructors working:

- `ApiState::default()` creates default event, connection, and statistics stores.
- `ApiState::new(event_store)` creates default connection and statistics stores.
- `ApiState::with_stores(event_store, connection_store)` creates default statistics store.

Add a constructor for tests and future composition that accepts all stores.

## Router

Add `statistics.rs` in `sql-lens-api` following the current `connections` and `sql_events` route-module style.

Merge it in `server::router_with_state`.

## Errors

Use `ApiEndpointError::bad_request` for invalid `window`.

Do not add a new API error shape.

## Compatibility

This is an additive API endpoint and additive storage snapshot field. Existing public methods keep their names and behavior.

## Tests

Add endpoint tests for:

- empty state returns zeroed values,
- populated state returns expected values,
- invalid window returns HTTP 400.

Extend live statistics tests for:

- empty percentile snapshot,
- populated percentile snapshot,
- sample window pruning.
