# Implement — Issue 080: Replay Preview API

## Steps

1. Read backend specs and existing API patterns for `sql_events`, `connections`,
   and `api_error`.
2. Add a `replay` API module with:
   - `REPLAY_PREVIEW_PATH`
   - request/response DTOs
   - preview source validation
   - conservative mutation classifier
3. Register replay routes in the API router and crate exports as needed.
4. Add tests for:
   - event ID preview with `expanded_sql`
   - event ID fallback to `original_sql`
   - raw SQL preview
   - mutation and read-only classification
   - bad source combinations
   - missing event ID
   - storage unchanged after preview
5. Update `API.md` with request/response examples and error behavior.
6. Validate:
   - `rtk cargo fmt --check`
   - `rtk cargo test -p sql-lens-api`
   - `rtk cargo test --workspace`
   - `rtk cargo clippy --workspace --all-targets -- -D warnings`

## Candidate Files

- `crates/sql-lens-api/src/replay.rs`
- `crates/sql-lens-api/src/server.rs`
- `crates/sql-lens-api/src/lib.rs`
- `API.md`

## Risk Points

- Keep SQL classification conservative; false positives are safer than false
  negatives for mutation warnings.
- Do not treat redacted expanded SQL as guaranteed executable SQL.
- Do not add execution, backend dialing, or replay job state in this task.
