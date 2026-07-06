# Define protocol adapter trait

## Goal

Issue 019: define the shared protocol adapter contract used by protocol-specific crates to observe traffic and emit normalized SQL capture events.

## User Value

SQL Lens needs a stable adapter boundary before implementing MySQL-compatible parsing. The proxy should be able to hand client/backend byte slices to a protocol adapter without knowing protocol-specific state or packet details.

## Background

- `sql-lens-core` owns protocol-neutral `SqlEvent`, `ConnectionInfo`, and `ProtocolName`.
- `sql-lens-capture` owns the runtime channel, but this trait should stay focused on the adapter contract.
- Issue 020 will add an adapter registry, so the trait should be object-safe.

## Requirements

- Implement the shared protocol adapter trait in `sql-lens-protocol`.
- The trait must be object-safe for future registry storage.
- The trait must observe client-to-backend bytes.
- The trait must observe backend-to-client bytes.
- The trait must emit `SqlEvent` values through an event emitter interface.
- The trait must support protocol-specific per-connection state.
- Keep the contract protocol-neutral.
- Add unit tests with a dummy adapter.

## Out Of Scope

- Adapter registry.
- MySQL packet parsing.
- Capture channel publishing.
- Storage.
- Proxy runtime integration.
- Async parsing.
- SQL redaction.

## Acceptance Criteria

- [ ] `sql-lens-protocol` depends only on `sql-lens-core`.
- [ ] `ProtocolAdapter` is object-safe.
- [ ] Adapter can create protocol-specific connection state.
- [ ] Adapter can observe client bytes.
- [ ] Adapter can observe backend bytes.
- [ ] Adapter can emit `SqlEvent` values through a capture event emitter trait.
- [ ] Observation result reports bytes observed and event count.
- [ ] Structured adapter errors exist without adding `thiserror` or `anyhow`.
- [ ] Unit tests cover client byte observation.
- [ ] Unit tests cover backend byte observation.
- [ ] Unit tests cover event emission.
- [ ] Unit tests cover protocol-specific state downcast.
- [ ] `cargo fmt --check` passes.
- [ ] `cargo check --workspace` passes.
- [ ] `cargo test --workspace` passes.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes.

## Open Questions

None blocking.
