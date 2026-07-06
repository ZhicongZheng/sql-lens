# Add protocol adapter registry

## Goal

Issue 020: add a protocol adapter registry that can register and resolve object-safe protocol adapters by protocol name.

## User Value

SQL Lens needs a single adapter selection boundary before runtime composition and MySQL adapter implementation. The runtime should be able to resolve the adapter for `mysql`, and future protocols can register without changing proxy code.

## Background

- Issue 019 added object-safe `ProtocolAdapter`.
- `sql-lens-config` currently owns config parsing/validation and must not depend on protocol crates.
- The registry should expose a structured unknown-adapter error that future config/runtime composition can map to user-facing config validation.

## Requirements

- Implement `ProtocolAdapterRegistry` in `sql-lens-protocol`.
- Registry can register adapter instances.
- Registry can resolve adapters by `ProtocolName`.
- Registry can report whether an adapter exists.
- Unknown adapter names return a structured error.
- Duplicate adapter names are rejected with a structured error.
- Keep adapter storage object-safe and shareable.
- Add unit tests for register, resolve, unknown adapter, duplicate registration, and lookup.

## Out Of Scope

- Config crate dependency on protocol.
- Runtime startup wiring.
- Built-in MySQL adapter registration.
- Protocol parsing.
- App-level config validation mapping.

## Acceptance Criteria

- [ ] Registry can register adapters.
- [ ] Registry can resolve adapters by protocol name.
- [ ] Resolved adapters are object-safe trait objects.
- [ ] Registry lookup can check whether an adapter exists.
- [ ] Unknown adapter names return structured `UnknownAdapter` error.
- [ ] Duplicate adapter names return structured `DuplicateAdapter` error.
- [ ] Tests cover adapter registration and lookup.
- [ ] Tests cover unknown adapter lookup.
- [ ] Tests cover duplicate registration.
- [ ] `cargo fmt --check` passes.
- [ ] `cargo check --workspace` passes.
- [ ] `cargo test --workspace` passes.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes.

## Open Questions

None blocking.
