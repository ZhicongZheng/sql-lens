# Plugin Runtime Implementation Plan

1. Decide and document the supported plugin artifact/loading boundary.
2. Add dispatcher ownership and lifecycle management.
3. Wire redacted connection and SQL event hook calls from runtime fan-out.
4. Add failure isolation, timeout, and shutdown handling.
5. Add tests for disabled mode, successful hooks, failures, and malformed configuration.

## Validation

- `cargo fmt --all -- --check`
- `cargo test -p sql-lens-plugin -p sql-lens-app`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
