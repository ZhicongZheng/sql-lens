# Runtime Redaction Implementation Plan

1. Add a config-to-core redaction policy mapping with unit tests.
2. Add policy-aware constructors or explicit policy injection to runtime-owned Ring Buffer and SQLite stores.
3. Ensure capture fan-out and WebSocket publication use the same redacted event boundary.
4. Add runtime integration tests for enabled custom policy and disabled policy.
5. Verify no default-policy bypass remains in app-owned storage paths.

## Validation

- `cargo fmt --all -- --check`
- `cargo test -p sql-lens-core -p sql-lens-storage -p sql-lens-app -p sql-lens-api`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
