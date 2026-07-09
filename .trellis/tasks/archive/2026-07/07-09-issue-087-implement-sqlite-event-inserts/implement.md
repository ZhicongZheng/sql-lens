# Implementation Plan

1. [x] Read storage specs and current SQLite schema module.
2. [x] Add `sqlite_event_store.rs` with `SqliteEventStore` and row structs.
3. [x] Serialize metadata and parameter values as JSON text.
4. [x] Insert events and parameters in one transaction.
5. [x] Re-export the store API from `sql-lens-storage`.
6. [x] Add tests for:
   - insert and readback of event scalar fields
   - parameter rows inserted
   - redaction before SQLite persistence
   - replacement/upsert or duplicate behavior decision
7. [x] Run narrow validation: `rtk cargo test -p sql-lens-storage`.
8. [x] Run broad validation: `rtk cargo fmt --check`, `rtk cargo test --workspace`, `rtk cargo clippy --workspace --all-targets -- -D warnings`.
