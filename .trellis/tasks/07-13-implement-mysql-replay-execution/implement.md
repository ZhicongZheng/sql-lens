# Guarded Replay Implementation Plan

1. [x] Define the execution request/response and API error contract.
2. [x] Add target resolution and `replay.enabled`/mutation confirmation checks.
3. [x] Implement a bounded MySQL-compatible execution client using existing runtime configuration.
4. [x] Map result rows, affected rows, and database errors without leaking secrets.
5. [x] Add unit tests for policy gates and runtime wiring; live backend coverage remains environment-dependent.
6. [ ] Enable the UI execute flow only after the backend contract is complete.

## Validation

- `cargo fmt --all -- --check`
- `cargo test -p sql-lens-api -p sql-lens-app`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
