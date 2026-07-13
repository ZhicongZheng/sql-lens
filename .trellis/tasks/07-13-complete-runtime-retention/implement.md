# Runtime Retention Implementation Plan

1. Add tests exposing timestamp representation and cutoff ordering across runtime and storage.
2. Implement Ring Buffer age deletion or document a bounded alternative if the store cannot support it without changing its contract.
3. Make SQLite age cleanup use the same canonical timestamp representation.
4. Decide and implement restart-only versus dynamic retention configuration; align validation and docs.
5. Define max-bytes behavior; reject it during config validation until a real implementation exists, or implement bounded file-size cleanup.
6. Add failure-isolation, count, and large-retention-set tests.

## Validation

- `cargo fmt --all -- --check`
- `cargo test -p sql-lens-storage -p sql-lens-app`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
