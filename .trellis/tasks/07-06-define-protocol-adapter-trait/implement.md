# Define protocol adapter trait implementation plan

## Steps

1. Add `sql-lens-core` dependency to `sql-lens-protocol`.
2. Implement protocol adapter contracts in `crates/sql-lens-protocol/src/lib.rs`.
3. Add unit-test helper types:
   - dummy adapter
   - dummy state
   - vector-backed capture event emitter
   - representative `SqlEvent`
4. Add unit tests for:
   - client byte observation
   - backend byte observation
   - event emission
   - state downcast
   - trait object usage
5. Update backend specs/agent docs for the protocol adapter contract.
6. Run validation:
   - `cargo fmt --check`
   - `cargo check --workspace`
   - `cargo test --workspace`
   - `cargo clippy --workspace --all-targets -- -D warnings`

## Constraints

- Keep `ProtocolAdapter` object-safe.
- Do not add `async-trait`.
- Do not add `thiserror`, `anyhow`, `serde_json`, `tokio`, or capture/storage/API/app dependencies.
- Do not parse MySQL packets in this task.

## Acceptance mapping

- Client/backend observation: `observe_client_bytes` and `observe_backend_bytes`.
- Capture emission: `CaptureEventEmitter`.
- Protocol-specific state: `ProtocolConnectionState`.
- Future registry compatibility: object-safe `ProtocolAdapter`.
