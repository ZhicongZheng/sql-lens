# Define protocol adapter trait design

## Boundary

Implement this in `crates/sql-lens-protocol`.

Allowed dependency:

- `sql-lens-core`

Do not depend on `sql-lens-capture` in this task. The protocol adapter contract should emit events into an abstract sink; runtime channel mapping can be added by composition code later.

## Public API

Planned types:

- `ProtocolAdapter`
  - object-safe trait
  - returns `ProtocolName`
  - creates boxed protocol-specific connection state
  - observes client and backend byte slices
- `ProtocolConnectionState`
  - type-erased state trait using `Any`
  - supports downcasting by concrete adapter implementations
- `ProtocolConnectionContext`
  - wraps the protocol-neutral `ConnectionInfo`
- `CaptureEventEmitter`
  - sink trait for emitted `SqlEvent` values
- `ProtocolObservation`
  - `bytes_observed`
  - `events_emitted`
- `ProtocolAdapterError`
  - invalid state
  - observation failure

## Object safety

Issue 020 needs a registry that can store heterogeneous adapters. Therefore `ProtocolAdapter` should not use associated types or generic methods.

Protocol-specific state is represented as `Box<dyn ProtocolConnectionState>`. Concrete adapters can downcast using `as_any` / `as_any_mut`.

## Event emission

Adapters emit already-normalized `SqlEvent` values through `CaptureEventEmitter`.

The emitter trait does not expose capture channel overload policy. That keeps parser code independent from runtime backpressure handling. A later composition task can adapt `CaptureEventPublisher` to `CaptureEventEmitter`.

## Tests

Use a dummy adapter and dummy state:

- client byte observation increments state and returns byte count
- backend byte observation increments state and returns byte count
- non-empty client bytes emit one `SqlEvent`
- state can be downcast from `dyn ProtocolConnectionState`
