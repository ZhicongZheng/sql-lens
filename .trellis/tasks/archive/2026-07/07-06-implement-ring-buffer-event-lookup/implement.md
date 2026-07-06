# Implement ring buffer event lookup implementation plan

## Steps

1. Add `RingBufferStore::get(&SqlEventId) -> Option<&SqlEvent>`.
2. Add tests for retained and evicted lookup.
3. Update backend spec with lookup contract.
4. Run validation:
   - `cargo fmt --check`
   - `cargo check --workspace`
   - `cargo test --workspace`
   - `cargo clippy --workspace --all-targets -- -D warnings`

## Constraints

- Do not add a lookup index yet.
- Do not clone the returned event.
- Do not change append or eviction semantics.
