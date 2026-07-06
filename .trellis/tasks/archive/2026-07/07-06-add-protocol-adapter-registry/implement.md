# Add protocol adapter registry implementation plan

## Steps

1. Add registry and registry error types to `crates/sql-lens-protocol/src/lib.rs`.
2. Store adapters in `HashMap<ProtocolName, Arc<dyn ProtocolAdapter>>`.
3. Add unit tests with dummy protocol adapters.
4. Update backend spec with registry contract.
5. Run validation:
   - `cargo fmt --check`
   - `cargo check --workspace`
   - `cargo test --workspace`
   - `cargo clippy --workspace --all-targets -- -D warnings`

## Constraints

- Keep registry in `sql-lens-protocol`.
- Do not modify `sql-lens-config` in this task.
- Do not add new dependencies.
- Do not register built-in protocol adapters yet.

## Acceptance mapping

- Register/resolve: `ProtocolAdapterRegistry::register` and `resolve`.
- Unknown adapter: `ProtocolAdapterRegistryError::UnknownAdapter`.
- Duplicate registration: `ProtocolAdapterRegistryError::DuplicateAdapter`.
- Object safety: registry stores `Arc<dyn ProtocolAdapter>`.
