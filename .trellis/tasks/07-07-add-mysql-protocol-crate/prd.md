# Add MySQL protocol crate

## Goal

Implement Issue 037: turn `sql-lens-protocol-mysql` from a placeholder crate into a buildable MySQL-compatible protocol adapter crate that can register a `mysql` adapter with the shared protocol registry.

## Requirements

- Add a minimal `MysqlProtocolAdapter`.
- The adapter reports protocol name `mysql`.
- The adapter creates protocol-specific connection state.
- The adapter can be registered in `ProtocolAdapterRegistry`.
- Observing client/backend bytes is a no-op parser foundation that returns byte counts and emits no events.
- Do not implement packet parsing, handshake parsing, authentication parsing, command parsing, or SQL event emission yet.
- Keep MySQL-specific state inside `sql-lens-protocol-mysql`.

## Acceptance Criteria

- [x] `sql-lens-protocol-mysql` builds as part of the workspace.
- [x] `MysqlProtocolAdapter::protocol_name()` returns `mysql`.
- [x] Adapter can be registered and resolved as `mysql` in `ProtocolAdapterRegistry`.
- [x] Adapter creates MySQL-specific connection state.
- [x] Client and backend byte observation returns observed byte counts and emits zero events.
- [x] Tests cover adapter registration.
- [x] `cargo fmt --check` passes.
- [x] `cargo test --workspace` passes.
- [x] `cargo clippy --workspace --all-targets -- -D warnings` passes.

## Out of Scope

- MySQL packet header parsing.
- MySQL handshake/authentication parsing.
- COM_QUERY or prepared statement parsing.
- Capture event emission from MySQL bytes.
- Runtime app composition or config wiring.
