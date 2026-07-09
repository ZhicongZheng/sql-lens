# Implementation Plan

1. [x] Read backend specs for directory structure, quality, and error-handling expectations.
2. [x] Inspect current `SqlEvent` construction paths in `sql-lens-protocol-mysql`.
3. [x] Add `fingerprint_sql` to `sql-lens-core` with focused unit tests for:
   - common `SELECT`
   - common `INSERT`
   - common `UPDATE`
   - common `DELETE`
   - whitespace and quoting edge cases
4. [x] Export the helper through `sql-lens-core::lib`.
5. [x] Wire MySQL event creation to populate `SqlEvent.fingerprint` using the helper.
6. [x] Update or add protocol tests that assert captured events include generated fingerprints.
7. [x] Run narrow validation:
   - `rtk cargo test -p sql-lens-core`
   - `rtk cargo test -p sql-lens-protocol-mysql`
8. [x] Run broad validation:
   - `rtk cargo fmt --check`
   - `rtk cargo test --workspace`
   - `rtk cargo clippy --workspace --all-targets -- -D warnings`

## Rollback Notes

The change should be isolated to core fingerprint helper/tests and adapter event construction. Rollback by removing the helper export and restoring event construction to `fingerprint: None`.
