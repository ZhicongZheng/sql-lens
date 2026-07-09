# Implementation Plan

1. [x] Read backend specs and existing API patterns.
2. [x] Add `export.rs` module to `sql-lens-api` with:
   - `EXPORT_PATH` constant
   - `routes()` returning the new route
   - `ExportQueryParams` (reuse filter parsing from `SqlEventListQueryParams`)
   - `export_sql_events` handler
   - Format enum (`json`, `ndjson`)
3. [x] Wire the new module into `sql-lens-api/src/lib.rs`.
4. [x] Register the route in `server.rs` router.
5. [x] Add tests for:
   - JSON export with filters
   - NDJSON export with filters
   - Redaction applied to exported events
   - Bounding at max limit
   - Invalid filter returns BAD_REQUEST
   - Invalid format returns BAD_REQUEST
6. [x] Run narrow validation: `rtk cargo test -p sql-lens-api`
7. [x] Run broad validation: `rtk cargo fmt --check`, `rtk cargo test --workspace`, `rtk cargo clippy --workspace --all-targets -- -D warnings`
