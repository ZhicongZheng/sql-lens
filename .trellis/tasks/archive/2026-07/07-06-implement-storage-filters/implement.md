# Implement storage filters implementation plan

## Steps

1. Add `SqlEventFilter` and `SqlEventFilterError` to `crates/sql-lens-storage/src/lib.rs`.
2. Add `filter: SqlEventFilter` to `RingBufferTimelineQuery`.
3. Update existing tests and test helpers to pass `SqlEventFilter::default()`.
4. Change `RingBufferStore::query_timeline` to return `Result<RingBufferTimelinePage, SqlEventFilterError>`.
5. Validate duration and timestamp ranges before scanning.
6. Apply filter matching during the newest-to-oldest timeline scan.
7. Preserve cursor pagination by producing `next_cursor` only when an older matching retained event exists.
8. Add tests for at least five filter combinations:
   - protocol + status
   - database type + database + user
   - min/max duration
   - SQL text
   - timestamp range
   - filtered cursor pagination if useful as the fifth or sixth test
9. Add tests for invalid duration range and invalid timestamp range.
10. Update backend storage contract in `.trellis/spec/backend/quality-guidelines.md`.
11. Run validation commands.

## Validation

```bash
rtk cargo fmt --check
rtk cargo check --workspace
rtk cargo test --workspace
rtk cargo clippy --workspace --all-targets -- -D warnings
```

## Constraints

- Do not add dependencies.
- Do not add API query parsing.
- Do not add WebSocket filters.
- Do not add SQLite/DuckDB behavior.
- Do not add secondary indexes.
- Do not parse SQL.
- Do not parse timestamps.
- Do not add fingerprint or client address filters in this task.

## Risk Points

- Existing `query_timeline` tests must be updated for the new `Result` return type without weakening assertions.
- `next_cursor` must be based on older matching events, not merely older retained events.
- Text filtering must not render SQL or inspect untrusted text as HTML.
- Timestamp range behavior is lexical and must be documented clearly.
