# Add live statistics counters implementation plan

## Steps

1. Read current `sql-lens-storage` code and backend storage contract.
2. Add `LiveStatistics`, `LiveStatisticsSnapshot`, and `LatencyBucketCount` to `crates/sql-lens-storage/src/lib.rs`.
3. Add fixed latency bucket constants inside the storage crate.
4. Implement `LiveStatistics::new` and `Default`.
5. Implement `record_sql_event` and `record_sql_event_at`.
6. Implement 60-second QPS tracking with `VecDeque<Instant>` and pruning.
7. Implement active connection tracking with `HashSet<ConnectionId>`.
8. Implement `snapshot` and `snapshot_at`.
9. Add focused unit tests for event counters, latency buckets, QPS window behavior, and active connection lifecycle behavior.
10. Update `.trellis/spec/backend/quality-guidelines.md` with the live statistics contract.
11. Mark PRD acceptance criteria complete after validation passes.

## Validation

```bash
rtk cargo fmt --check
rtk cargo check --workspace
rtk cargo test --workspace
rtk cargo clippy --workspace --all-targets -- -D warnings
```

## Constraints

- Do not add dependencies.
- Do not add a new crate.
- Do not add REST API, WebSocket, frontend, or runtime wiring.
- Do not depend on capture, proxy, API, or app crates.
- Do not infer active connections from SQL events.
- Do not parse `SqlEvent.timestamp`.
- Do not implement p50/p95/p99 or top-N rankings.

## Risk Points

- QPS tests must use deterministic `Instant` values, not wall-clock sleeps.
- QPS pruning must keep memory bounded.
- Active connection open/close must be idempotent enough for lifecycle replay or duplicate calls.
- Latency bucket boundaries must be clear and stable because future API/UI code may expose them.
