# Retention Enforcement Implementation Plan

## Prerequisites

- [ ] Issue 089 complete (retention configuration mechanism exists)
- [ ] Issue 112 complete (retention configuration stored and retrievable)
- [ ] Storage layer supports time-based deletion

## Implementation Checklist

### Phase 1: Core Enforcement Logic

1. **Define RetentionEnforcer struct and trait bounds**
   - File: `crates/sql-lens-app/src/retention.rs` (new)
   - Depends on: `AppConfig`, `Storage` trait

2. **Implement cutoff calculation**
   - Read global retention from config
   - Read per-table overrides
   - Compute `cutoff = now - retention_period` per source

3. **Implement enforcement loop**
   - Iterate over all configured tables/queries
   - Call `storage.delete_before(table, cutoff)`
   - Collect metrics (rows deleted, duration)

4. **Add audit logging**
   - Log enforcement start/end with counts
   - Log per-table deletion counts

### Phase 2: Scheduling

5. **Integrate with tokio runtime scheduler** [x]
   - Spawn background task on app startup
   - Configurable interval from `retention.enforcement_interval`
   - Graceful shutdown handling

6. **Add enable/disable toggle** [x]
   - Config flag: `retention.enforcement_enabled`
   - Skip enforcement loop if disabled

### Phase 3: Error Handling & Robustness

7. **Handle concurrent access**
   - Use appropriate locking or transaction isolation
   - Ensure capture writes are not blocked

8. **Chunk large deletions**
   - If deletion count exceeds threshold, batch in chunks
   - Prevents long-running transactions

9. **Error recovery**
   - Log errors, continue with next table
   - Do not propagate errors to crash app

### Phase 4: Testing

10. **Unit tests for cutoff calculation**
    - Test global default
    - Test per-table override precedence

11. **Integration test for enforcement**
    - Seed data with varying timestamps
    - Run enforcer
    - Assert old data deleted, new data preserved

## Risky Files

- `crates/sql-lens-app/src/lib.rs` — app startup wiring
- Storage implementation files — verify `delete_before` contract

## Validation Commands

```bash
# After implementation
cargo test retention
cargo clippy
cargo build --release
```

## Rollback Points

- If enforcement causes capture stalls: disable via config flag
- If deletion performance is poor: add chunking or reduce frequency

## Open Implementation Questions

None at this time — pending code exploration during implementation.
