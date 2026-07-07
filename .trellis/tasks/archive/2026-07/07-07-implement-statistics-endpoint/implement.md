# Implement statistics endpoint plan

## Checklist

- [x] Read backend specs before editing.
- [x] Extend `LiveStatisticsSnapshot` with latency percentile data.
- [x] Add bounded recent latency samples to `LiveStatistics`.
- [x] Add or update `LiveStatistics` unit tests for percentiles and pruning.
- [x] Add `LiveStatistics` to `ApiState` with backward-compatible constructors.
- [x] Add `statistics.rs` API route module.
- [x] Merge statistics routes in `server::router_with_state`.
- [x] Add API tests for empty, populated, and invalid-window cases.
- [x] Update backend spec to include the new percentile contract.
- [x] Run `rtk cargo fmt --check`.
- [x] Run `rtk cargo test --workspace`.
- [x] Run `rtk cargo clippy --workspace --all-targets -- -D warnings`.

## Validation Commands

```bash
rtk cargo fmt --check
rtk cargo test --workspace
```

## Risk Notes

- Percentiles must be computed from retained recent samples, not inferred from coarse latency buckets.
- Empty state must avoid divide-by-zero and return `0.0` values.
- Existing API tests must keep working through the current `ApiState` constructors.
- This task should not implement documented future filters until a storage-backed statistics query layer exists.

## Rollback Points

- If API state constructor changes cause broad churn, keep existing constructors and add only a new all-stores constructor.
- If percentile snapshot fields create compatibility trouble, keep them additive and avoid renaming existing fields.
