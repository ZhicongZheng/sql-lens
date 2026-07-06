# Implement ring buffer timeline query implementation plan

## Steps

1. Add internal `RingBufferEntry` with append sequence.
2. Update append, snapshot, and get to use entries while preserving public behavior.
3. Add timeline query, cursor, and page types.
4. Add tests for ordering, limit, cursor pagination, cursor stability, and snapshot behavior.
5. Update backend spec with timeline query contract.
6. Run validation:
   - `cargo fmt --check`
   - `cargo check --workspace`
   - `cargo test --workspace`
   - `cargo clippy --workspace --all-targets -- -D warnings`

## Constraints

- Do not add filters.
- Do not expose internal sequence on `SqlEvent`.
- Do not add persistent cursor serialization.
- Do not add indexes until query requirements need them.
