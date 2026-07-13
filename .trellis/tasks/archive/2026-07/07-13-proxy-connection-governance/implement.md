# Proxy Governance Implementation Plan

1. [x] Add a session admission and tracking primitive with tests for capacity, release, and shutdown.
2. [x] Pass the configured connection and idle timeout values from app startup into the proxy runtime.
3. [x] Wrap accepted client handling and forwarding in tracked session tasks.
4. [x] Apply idle timeout around session activity and preserve lifecycle finalization on timeout.
5. [x] Replace detached session shutdown with bounded drain and abort handling.
6. [x] Add runtime tests for max connections, idle timeout, graceful drain, and forced timeout.

## Validation

- `cargo fmt --all -- --check`
- `cargo test -p sql-lens-proxy`
- `cargo test -p sql-lens-app`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
