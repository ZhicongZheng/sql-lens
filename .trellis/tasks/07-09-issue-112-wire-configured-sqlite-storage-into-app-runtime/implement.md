# Issue 112 Implementation Plan

1. Read backend storage/runtime specs.
2. Add path-based `SqliteEventStore::open`.
3. Add app runtime persistent sink types and SQLite worker.
4. Build sink from `SqlLensConfig.storage`.
5. Fan out classified events to ring buffer/statistics/WebSocket and SQLite sink.
6. Add tests:
   - SQLite store opens file path and migrates schema.
   - runtime rejects empty SQLite path.
   - runtime starts with SQLite storage and persists a captured/test event.
   - ring-buffer-only runtime still has no persistent sink.
7. Update specs if runtime storage contract changes.
8. Validate:
   - `rtk cargo fmt --check`
   - `rtk cargo test -p sql-lens-storage`
   - `rtk cargo test -p sql-lens-app`
   - `rtk cargo test --workspace`
   - `rtk cargo clippy --workspace --all-targets -- -D warnings`

