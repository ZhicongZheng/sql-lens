# Capture COM_QUERY timing design

## Boundary

Implement in `crates/sql-lens-protocol-mysql` with normalized `SqlEvent` values from `sql-lens-core`.

Do not import `sql-lens-capture`, storage, API, proxy, or async runtime crates. The adapter emits through the existing `CaptureEventEmitter` trait only.

## Current State

- `MysqlConnectionState` reaches `Authenticated`.
- `observe_client_bytes` can parse `COM_QUERY` and store `last_client_command`.
- `observe_backend_bytes` can observe authentication OK/ERR while phase is `ClientHandshakeSeen`.
- No pending query state or SQL event emission exists in the MySQL adapter.

## Time Source

The adapter needs a small time source because `SqlEvent` requires both display timestamps and a millisecond duration.

Use a MySQL-local clock abstraction:

```rust
pub struct MysqlObservationTime {
    pub timestamp: sql_lens_core::Timestamp,
    pub monotonic: std::time::Instant,
}

pub trait MysqlObservationClock: std::fmt::Debug + Send + Sync {
    fn now(&self) -> MysqlObservationTime;
}
```

`MysqlProtocolAdapter::new()` should use a system clock implemented with standard library only. Tests can use a deterministic manual clock through a test-only or public constructor such as:

```rust
impl MysqlProtocolAdapter {
    pub fn with_clock(clock: std::sync::Arc<dyn MysqlObservationClock>) -> Self;
}
```

No `time`, `chrono`, or `uuid` dependency should be introduced for this task.

## Pending Query State

Extend MySQL-specific state:

```rust
pub struct MysqlPendingQuery {
    pub command: MysqlClientCommand,
    pub started_at: sql_lens_core::Timestamp,
    pub started_monotonic: std::time::Instant,
}

impl MysqlConnectionState {
    pub fn pending_query(&self) -> Option<&MysqlPendingQuery>;
}
```

On valid `COM_QUERY` after `Authenticated`, replace any existing pending query with the new query. This keeps first implementation simple; later result-set handling can refine multi-response behavior.

## Backend Terminal Detection

For this issue only:

- Payload first byte `0x00` means terminal OK.
- Payload first byte `0xff` means terminal ERR.
- Any other backend response keeps the pending query open and emits no event.

Detailed OK/ERR parsing belongs to Issues 045 and 046.

## Event Construction

When finalizing:

- `kind`: `SqlEventKind::Query`.
- `status`: `CaptureStatus::Ok` for OK, `CaptureStatus::Error` for ERR.
- `duration`: elapsed milliseconds from pending start monotonic time to end monotonic time.
- `timings.started_at`: pending start timestamp.
- `timings.ended_at`: finalization timestamp.
- `original_sql`: pending command SQL.
- `normalized_sql`, `expanded_sql`, `fingerprint`: `None`.
- `parameters`: empty.
- `result`: `None` until OK summary parsing.
- `error`: minimal sanitized error summary for ERR if needed; detailed parsing later.
- `metadata`: protocol `mysql` with at least command name and command sequence ID.

Use connection fields copied from `ProtocolConnectionContext`, so `MysqlConnectionState` should retain the context's `ConnectionInfo` or the needed fields at creation time.

Event IDs can be deterministic process-local strings derived from connection ID and an incrementing per-state query counter, for example `conn_1_query_1`. Do not introduce UUID.

## Adapter Behavior

`observe_client_bytes`:

- Continues handshake and command parsing behavior.
- On valid `COM_QUERY` after `Authenticated`, stores pending query timing and emits zero events.

`observe_backend_bytes`:

- Continues initial handshake and auth result behavior.
- When phase is `Authenticated` and a pending query exists, attempts terminal response detection.
- On terminal OK/ERR, emits one event and clears pending query.
- On unsupported response, malformed packet, or missing pending query, emits zero events and remains non-fatal.

`ProtocolObservation.events_emitted` must match the number of emitted events.

## Tests

Tests should use a deterministic clock and existing packet helpers.

Adapter tests:

- `COM_QUERY` after authentication creates pending query and emits zero events.
- Backend OK finalizes pending query, emits one OK event, records duration, and clears pending state.
- Backend ERR finalizes pending query, emits one error event, records duration, and clears pending state.
- Backend OK/ERR without pending query emits zero events.
- Unsupported backend response with pending query emits zero events and keeps pending state.

## Rollback

If event construction is too broad, keep pending timing state and terminal detection but defer event emission. Do not add storage/API dependencies to work around missing runtime composition.
