# Add protocol adapter registry design

## Boundary

Implement registry types in `crates/sql-lens-protocol`.

Allowed dependencies remain:

- `sql-lens-core`
- standard library only

Do not make `sql-lens-config` depend on `sql-lens-protocol` in this task.

## Public API

Planned types:

- `ProtocolAdapterRegistry`
  - stores adapters keyed by `ProtocolName`
  - `register(adapter)`
  - `resolve(protocol)`
  - `contains(protocol)`
  - `len`
  - `is_empty`
- `ProtocolAdapterRegistryError`
  - `DuplicateAdapter { protocol }`
  - `UnknownAdapter { protocol }`

## Storage

Use `Arc<dyn ProtocolAdapter>` internally so resolved adapters can be shared by runtime tasks without cloning concrete adapter state.

## Unknown adapter behavior

`resolve` returns `ProtocolAdapterRegistryError::UnknownAdapter`.

This is the protocol-layer source error. A later runtime/config composition task can map it to the config validation display shape without introducing a reverse dependency from config to protocol.

## Tests

Use dummy adapters from the protocol crate test module:

- register one adapter and resolve it
- `contains` reports true/false
- resolving unknown adapter returns `UnknownAdapter`
- registering duplicate protocol name returns `DuplicateAdapter`
