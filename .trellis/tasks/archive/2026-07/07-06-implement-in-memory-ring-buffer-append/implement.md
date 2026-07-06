# Implement in-memory ring buffer append implementation plan

## Steps

1. Add `sql-lens-core` dependency to `sql-lens-storage`.
2. Implement `RingBufferStore`, `RingBufferAppendOutcome`, and `RingBufferStats`.
3. Add test helper for representative `SqlEvent`.
4. Add unit tests for append, capacity enforcement, oldest eviction, stats, and non-zero capacity.
5. Update backend spec with ring buffer append contract.
6. Run validation:
   - `cargo fmt --check`
   - `cargo check --workspace`
   - `cargo test --workspace`
   - `cargo clippy --workspace --all-targets -- -D warnings`

## Constraints

- Keep this synchronous and in-memory.
- Do not add lookup index yet.
- Do not add async/runtime/database dependencies.
- Do not mutate `SqlEvent`.

## Acceptance mapping

- Append: `RingBufferStore::append`.
- Capacity: `NonZeroUsize` plus `VecDeque` length enforcement.
- Oldest eviction: `pop_front`.
- Stats: `RingBufferStats`.
