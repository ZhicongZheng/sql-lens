# Add WebSocket filters design

## Boundary

Implement in `crates/sql-lens-api`, focused on `/ws/sql` subscription parsing and per-subscriber event filtering.

This task does not change proxy capture, storage persistence, REST query APIs, frontend code, or runtime wiring.

## Current State

- `/ws/sql` accepts WebSocket upgrades and sends an initial ping heartbeat.
- Clients must send `{"type":"subscribe","version":1}` before receiving live events.
- Live events are broadcast through `ApiState::sql_event_broadcaster()`.
- `sql_event.created` payloads reuse `SqlEventSummaryResponse`.
- Invalid non-filter subscription messages are currently ignored while the socket waits for a valid subscribe.

## Subscribe Contract

Supported subscribe message:

```json
{
  "type": "subscribe",
  "version": 1,
  "filters": {
    "protocol": "mysql",
    "status": ["ok", "error"],
    "database": "app",
    "min_duration_ms": 10,
    "max_duration_ms": 500
  }
}
```

Rules:

- `filters` is optional.
- Missing `filters` means match all future events.
- `protocol` is an exact string match against `SqlEvent.protocol`.
- `database` is an exact string match against `SqlEvent.database`.
- `status` is an array of allowed strings: `ok`, `slow`, `error`, `unknown`.
- `status` must not be empty when present.
- `min_duration_ms` and `max_duration_ms` are inclusive `u64` values.
- If both duration bounds are present, `min_duration_ms <= max_duration_ms`.
- Unknown fields inside `filters` are invalid.
- Filters are protocol-neutral and must not add MySQL-specific matching behavior.

## Error Contract

Invalid filters return a WebSocket text message:

```json
{
  "type": "subscription.error",
  "version": 1,
  "payload": {
    "code": "INVALID_FILTER",
    "message": "invalid subscription filter",
    "field": "filters.status"
  }
}
```

Behavior:

- Send the error message on the same socket.
- Keep the socket open.
- Continue waiting for a valid `subscribe` message.
- Do not emit `sql_event.created` until a valid subscription is accepted.

Malformed JSON, wrong top-level `type`, or wrong top-level `version` stays aligned with Issue 035 and is ignored while waiting for a valid subscribe. This task only adds explicit errors for invalid filters within an otherwise recognizable subscribe request.

## Filter Implementation

Add a private WebSocket filter type, for example:

```rust
struct SqlEventSubscriptionFilter {
    protocol: Option<ProtocolName>,
    statuses: Option<Vec<CaptureStatus>>,
    database: Option<String>,
    min_duration: Option<DurationMillis>,
    max_duration: Option<DurationMillis>,
}
```

Reasoning:

- REST `SqlEventFilter` supports a single status; WebSocket subscriptions must support multiple statuses.
- Keeping a WebSocket-local predicate avoids changing storage query contracts.
- The predicate still uses core domain types and field semantics consistent with REST filters.

Matching behavior:

- All provided filter fields are combined with AND semantics.
- `status` values are combined with OR semantics inside that field.
- Non-matching events are skipped silently for that subscriber.
- Filtering is per subscriber; one subscriber's filters do not affect other subscribers.

## Data Flow

```text
client subscribe with filters
        |
        v
parse filters -> SqlEventSubscriptionFilter
        |
        v
subscribe to ApiState broadcaster
        |
        v
for each SqlEvent:
  if filter.matches(event):
      send sql_event.created
  else:
      skip
```

## Tests

Focused tests should cover:

- Matching and non-matching `protocol`.
- Multiple `status` values.
- Matching and non-matching `database`.
- Matching and non-matching duration range.
- Unfiltered subscription still receives all events.
- Invalid status returns `subscription.error`.
- Invalid duration range returns `subscription.error`.
- After an error, a later valid subscribe works on the same socket.

## Compatibility

This is additive:

- Existing unfiltered subscription behavior remains valid.
- Existing `sql_event.created` message shape does not change.
- Existing WebSocket upgrade and heartbeat behavior remains valid.
- Current REST query behavior does not change.

## Rollback

If live socket filter tests become flaky, keep unit coverage for parsing/matching and one endpoint smoke test for error framing, then split the broader live delivery matrix into a follow-up. Do not introduce storage or app runtime changes to compensate.
