# Protocol Registry Runtime Implementation Plan

1. Add app-side built-in adapter registration and protocol-name resolution.
2. Refactor target runtime configuration to carry the selected adapter.
3. Replace hard-coded MySQL construction in the forwarding path while preserving observation behavior.
4. Add unsupported-protocol startup tests and MySQL regression tests.
5. Update protocol discovery output only if it currently contradicts runtime support.

## Validation

- `cargo fmt --all -- --check`
- `cargo test -p sql-lens-protocol -p sql-lens-protocol-mysql -p sql-lens-app`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
