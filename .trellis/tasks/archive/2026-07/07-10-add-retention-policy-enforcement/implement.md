# Retention Policy Enforcement Implementation Plan

## Prerequisites

- [x] Issue 021 complete (ring buffer with `enforce_max_events`)
- [x] Issue 087 complete (SQLite with `delete_events_older_than` and `enforce_max_events`)
- [x] Storage layer methods implemented and unit-tested

## Implementation Checklist

### Phase 1: Verification (No Code Changes Expected)

1. **Verify ring buffer cleanup exists and tested**
   - File: `crates/sql-lens-storage/src/ring_buffer.rs`
   - Method: `enforce_max_events()` at line 68
   - Tests: `ring_buffer_retention_*` at line 1106+
   - Status: ✅ Confirmed

2. **Verify SQLite cleanup exists and tested**
   - File: `crates/sql-lens-storage/src/sqlite_event_store.rs`
   - Methods:
     - `delete_events_older_than()` at line 307 (age-based)
     - `enforce_max_events()` at line 335 (count-based)
   - Tests: `sqlite_retention_*` at line 1192+
   - Status: ✅ Confirmed

3. **Verify RetentionConfig structure**
   - File: `crates/sql-lens-config/src/model.rs`
   - Struct: `RetentionConfig` at line 174
   - Fields: `max_age`, `max_events`, `max_bytes`, `drop_policy`
   - Default tests: line 84-98
   - Status: ✅ Confirmed

### Phase 2: Documentation & Scope Clarification

4. **Update prd.md with scope decisions**
   - Global-only retention (per-table deferred)
   - Synchronous methods (async scheduling deferred)
   - Storage layer focus (app runtime integration deferred to Issue 117)
   - Status: ✅ Complete

5. **Create design.md**
   - Architecture boundaries
   - Contracts for cleanup methods
   - Trade-offs documented
   - Status: ✅ Complete (this document)

### Phase 3: Gap Analysis (If Any)

6. **Check for missing test coverage**
   - Run: `cargo test -p sql-lens-storage retention`
   - Verify all retention tests pass
   - Identify any edge cases not covered (e.g., zero max_events, empty store)

7. **Check for missing documentation**
   - Ensure cleanup methods have rustdoc comments
   - Verify `RetentionConfig` TOML examples exist

## Risky Files

- None — task is verification-only, no modifications to production code expected
- If gap analysis reveals missing tests, add to test files only

## Validation Commands

```bash
# Verify all retention-related tests pass
cargo test -p sql-lens-storage retention

# Full storage crate test suite
cargo test -p sql-lens-storage

# Config crate tests (retention defaults)
cargo test -p sql-lens-config retention

# Workspace-wide checks
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo check --workspace
```

## Rollback Points

- No code changes expected — no rollback needed
- If test gaps identified, new tests can be added without affecting existing behavior

## Open Implementation Questions

None — task scope is confirmation of existing implementation.
