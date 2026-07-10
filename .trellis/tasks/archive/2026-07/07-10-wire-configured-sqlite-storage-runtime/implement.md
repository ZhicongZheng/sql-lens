# SQLite Storage Wiring Implementation Plan

## Prerequisites

- [x] Issue 087 complete (SQLite event storage with `SqliteEventStore`, `EventPersistence::sqlite`)
- [x] Issue 109 complete (app runtime startup with `start_minimal_mysql_runtime_with_runtime_storage`)

## Implementation Checklist

### Phase 1: Verification (No Code Changes Expected)

1. **Verify RuntimeStorage::from_config handles StorageType::Sqlite**
   - File: `crates/sql-lens-app/src/lib.rs:381-418`
   - Creates `SqliteEventStore` at configured path
   - Creates separate reader connection for API queries
   - Initializes `EventPersistence::sqlite(store)` with background worker
   - Status: ✅ Confirmed

2. **Verify error handling for invalid/missing path**
   - `sqlite_storage_path()` at line 444 validates path requirement
   - Error: `MinimalMysqlRuntimeError::StorageConfig("storage.path is required...")`
   - SQLite open failure: `MinimalMysqlRuntimeError::SqliteStorage { path, source }`
   - Status: ✅ Confirmed

3. **Verify DuckDB unsupported error**
   - Line 415-417 returns explicit "not supported yet" error
   - Status: ✅ Confirmed

4. **Verify capture pipeline writes to both storages**
   - `store_sql_events()` accepts `EventPersistence` parameter
   - Ring buffer always updated via `ApiState`
   - SQLite persistence via async worker (non-blocking)
   - Status: ✅ Confirmed

### Phase 2: Test Validation

5. **Run existing SQLite persistence tests**
   - `store_sql_events_persists_to_sqlite_worker` (line 1550)
   - `sqlite_worker_insert_failure_does_not_stop_capture_state` (line 1585)
   - Verify both tests pass with temporary database paths

6. **Run ring-buffer-only tests**
   - Verify default behavior unchanged when `StorageConfig::default()` used
   - Check `RuntimeStorage` test at line 1629

### Phase 3: Documentation

7. **Update prd.md with scope decisions**
   - Verification-only task (implementation exists)
   - All AC requirements satisfied by existing code
   - Status: ✅ Complete

8. **Create design.md**
   - Architecture boundaries documented
   - Contracts for RuntimeStorage and EventPersistence
   - Trade-offs documented
   - Status: ✅ Complete (this document)

## Risky Files

- None — task is verification-only, no production code modifications expected
- If test gaps identified, add to test files in `crates/sql-lens-app/src/lib.rs`

## Validation Commands

```bash
# Verify all storage-related tests pass
cargo test -p sql-lens-app storage
cargo test -p sql-lens-app sqlite

# Full app crate test suite
cargo test -p sql-lens-app

# Workspace-wide quality checks
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo check --workspace
```

## Rollback Points

- No code changes expected — no rollback needed
- If test failures identified, existing implementation may need bug fix (out of scope for Issue 112)

## Open Implementation Questions

None — task scope is confirmation of existing implementation.

