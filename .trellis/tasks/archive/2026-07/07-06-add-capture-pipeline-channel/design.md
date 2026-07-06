# Add capture pipeline channel design

## Boundary

Create `crates/sql-lens-capture` as a small runtime primitive crate.

Dependencies:

- `sql-lens-core` for `SqlEvent`.
- `tokio` with `sync` for `mpsc`.

Do not depend on proxy, protocol, storage, API, plugin, app, database, HTTP, or exporter crates.

## Public API

Planned types:

- `CapturePipelineConfig`
  - `capacity: NonZeroUsize`
  - `overload_policy: CaptureOverloadPolicy`
- `CaptureOverloadPolicy`
  - `DropNewest`
  - `RejectNew`
- `CaptureEventPublisher`
  - non-blocking `publish(SqlEvent)`
  - shared dropped counter
- `CaptureEventReceiver`
  - async `recv()`
  - future fan-out task input
- `CapturePublishOutcome`
  - `Enqueued`
  - `Dropped`
- `CapturePublishError`
  - `Full { event }`
  - `Closed { event }`
- `CapturePipelineStats`
  - `dropped_events`

## Overload semantics

Publishing must call `try_send`.

- `DropNewest`
  - If the channel is full, drop the incoming event.
  - Increment `dropped_events`.
  - Return `Ok(CapturePublishOutcome::Dropped)`.
- `RejectNew`
  - If the channel is full, increment `dropped_events`.
  - Return `Err(CapturePublishError::Full { event })` so the caller may inspect or handle it.
- `Closed`
  - If the receiver is closed, return `Err(CapturePublishError::Closed { event })`.
  - Do not count it as overload.

## Capacity

Use `NonZeroUsize` in `CapturePipelineConfig` so a zero-capacity channel cannot be constructed.

This task does not add TOML fields. Runtime mapping from `SqlLensConfig` to capture pipeline config can be added when `sql-lens-app` composes services.

## Tests

Unit tests live in `crates/sql-lens-capture/src/lib.rs`:

- publisher enqueues an event and receiver reads it
- drop-newest overload increments dropped counter and only the first event is received
- reject-new overload returns the event and increments dropped counter
- closed receiver returns structured closed error
