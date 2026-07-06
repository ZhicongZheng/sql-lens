# Implement ring buffer event lookup design

## Boundary

Implement in `crates/sql-lens-storage`.

## Public API

Add:

```rust
impl RingBufferStore {
    pub fn get(&self, id: &SqlEventId) -> Option<&SqlEvent>;
}
```

## Design

Use linear scan over the retained `VecDeque`.

Reasoning:

- Issue 022 only requires correctness.
- Capacity is bounded.
- An ID index can be added in a later optimization or when API query behavior needs it.
- Avoiding an index keeps eviction consistency simple for now.

## Tests

- Append two events, retrieve the first by ID.
- Fill capacity one, evict the first event, verify first lookup returns `None` and second lookup returns `Some`.
