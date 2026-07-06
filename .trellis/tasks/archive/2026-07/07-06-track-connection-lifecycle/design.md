# Track connection lifecycle design

## Scope

Add proxy-local connection lifecycle tracking inside `sql-lens-proxy`.

This task does not add storage, API exposure, protocol parsing, app runtime wiring, timestamps from a time crate, or UUID generation.

## Types

- `ConnectionLifecycleIdGenerator`
  - Owns monotonic in-process connection ID generation.
  - Produces `sql_lens_core::ConnectionId`.
- `ConnectionLifecycleRecord`
  - Owns the current `sql_lens_core::ConnectionInfo`.
  - Stores optional failure context for backend dial and forwarding failures.
  - Exposes small transition methods.
- `ConnectionLifecycleFailure`
  - Protocol-neutral proxy-local failure reason.
  - Maps backend dial failure kinds without exposing protocol-specific details.

## State transitions

The first supported transitions are:

1. Accepted client creates a `ConnectionLifecycleRecord` with `ConnectionState::Created`.
2. Successful backend dial moves the record to `ConnectionState::BackendConnected`.
3. Forwarding completion first moves to `ConnectionState::Closing`, updates byte counters, then moves to `ConnectionState::Closed`.
4. Backend dial failure moves the record to `ConnectionState::Failed`.
5. Forwarding failure moves the record to `ConnectionState::Failed` and preserves any available byte counters.

## Core model usage

Use existing protocol-neutral core types:

- `ConnectionId`
- `ConnectionInfo`
- `ConnectionState`
- `ProtocolName`
- `DatabaseType`
- `Timestamp`

Timestamp values are simple strings for now because `sql-lens-core` already owns a `Timestamp(String)` newtype and this task must not add a time dependency.

## Testing

Add unit tests in `crates/sql-lens-proxy/src/lib.rs`:

- normal forwarding summary transitions to closed and updates bytes
- backend dial failure transitions to failed with failure context
- ID generator produces stable sequential IDs

Validation:

- `cargo fmt --check`
- `cargo check --workspace`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
