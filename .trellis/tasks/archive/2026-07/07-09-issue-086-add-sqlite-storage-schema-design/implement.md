# Implementation Plan

1. [x] Read backend/storage specs and SQLite library docs.
2. [x] Add `rusqlite` dependency to `sql-lens-storage`.
3. [x] Add `sqlite_schema.rs` with schema constants and `apply_sqlite_schema`.
4. [x] Re-export the schema API from `sql-lens-storage/src/lib.rs`.
5. [x] Add tests using an in-memory SQLite connection:
   - migration applies to an empty database
   - required tables exist
   - recommended indexes exist
   - schema version row exists
   - migration is idempotent
6. [x] Run narrow validation: `rtk cargo test -p sql-lens-storage`.
7. [x] Run broad validation: `rtk cargo fmt --check`, `rtk cargo test --workspace`, `rtk cargo clippy --workspace --all-targets -- -D warnings`.
