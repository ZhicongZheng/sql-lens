# Design

## Boundary

Implement fingerprinting in `sql-lens-core` because it is protocol-neutral domain behavior and `SqlEvent.fingerprint` already lives there. Protocol adapters can call the helper when building events, but core must not depend on MySQL-specific crates.

Do not introduce a SQL parser dependency for this foundation task. The first implementation should be a deterministic scanner that handles common literals and whitespace while preserving statement shape.

## Public Contract

Expose a small helper from core:

```rust
pub fn fingerprint_sql(sql: &str) -> String;
```

The helper returns a lower-cased, whitespace-normalized string with supported literals replaced by `?`. It should be total over arbitrary input and never return `Result`.

## Normalization Rules

- ASCII whitespace collapses to a single space outside quoted literals.
- Leading and trailing whitespace is removed.
- Single-quoted and double-quoted string literals become `?`.
- Escaped quote characters inside quoted strings are consumed best-effort.
- Decimal integers, decimal floats, and hexadecimal numeric literals become `?`.
- `NULL`, `TRUE`, and `FALSE` keywords become `?`.
- Identifiers, punctuation, comments, operators, and other tokens are preserved except for lower-casing ASCII letters.

## Event Integration

The MySQL adapter should call `fingerprint_sql` when constructing SQL events. Prefer expanded SQL when it is the user-visible executed statement; otherwise use the original/template SQL. This keeps the generated fingerprint aligned with API filtering and future grouping behavior.

Existing tests that build `SqlEvent` directly may keep literal fixture fingerprints where they are testing API/storage behavior rather than the fingerprint algorithm.

## Compatibility

This change only fills an existing optional field. It should not change REST response shape, WebSocket envelope shape, storage query parameters, or replay preview behavior.

## Validation

- Unit tests in `sql-lens-core` for fingerprint normalization.
- Adapter tests in `sql-lens-protocol-mysql` for populated event fingerprints.
- Existing workspace tests and clippy before commit.
