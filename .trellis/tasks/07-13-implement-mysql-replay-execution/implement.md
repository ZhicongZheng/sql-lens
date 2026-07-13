# Guarded Replay Implementation Plan

1. Define the execution request/response and API error contract.
2. Add target resolution and `replay.enabled`/mutation confirmation checks.
3. Implement a bounded MySQL-compatible execution client using existing runtime configuration.
4. Map result rows, affected rows, and database errors without leaking secrets.
5. Add unit tests for policy gates and integration tests against a live test database when available.
6. Enable the UI execute flow only after the backend contract is complete.

## Validation

- `cargo fmt --all -- --check`
- `cargo test -p sql-lens-api -p sql-lens-app`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
