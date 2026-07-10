# Capture Pipeline Runtime Fan-Out Design

## Architecture

`sql-lens-app` will own one `CapturePipeline` for the complete runtime. Every
MySQL proxy target receives a clone of its `CaptureEventPublisher`; the runtime
spawns one task that owns the corresponding `CaptureEventReceiver`.

```text
MySQL adapter -> CaptureEventPublisher::publish (try_send)
                    |
                    v
            CaptureEventReceiver task
                    |
                    +-> classify once
                    +-> WebSocket broadcaster
                    +-> LiveStatistics
                    +-> RingBufferStore
                    +-> optional SQLite persistence queue
```

The producer only logs non-success outcomes. It never awaits the consumer or
any storage/API work. The consumer preserves the current fan-out order and
uses the existing `EventPersistence` queue for SQLite.

## Configuration

Add config-owned types to `sql-lens-config`:

```toml
[capture]
capacity = 1024
overload_policy = "drop_newest"
```

- `capacity` is a `u64` startup value. Validation rejects zero before runtime
  conversion to `NonZeroUsize`.
- `overload_policy` is a config-owned enum with `drop_newest` and `reject_new`.
  `sql-lens-app` maps it to the capture crate enum, keeping config independent
  from capture implementation types.
- The configuration default is `1024` / `drop_newest`. The existing runtime
  convenience constructors use the same default.

## Publication Outcomes

- `Enqueued`: no producer-side action.
- `Dropped` or `Full`: a full pipeline increments `dropped_events`; app logs a
  warning without SQL text or parameter values.
- `Closed`: capture crate increments a new `closed_events` counter; app logs a
  warning and continues packet forwarding.

`CapturePipelineStats` gains the additive `closed_events` field. No REST or
WebSocket schema changes are part of this task.

## Lifecycle

The runtime stores a consumer-shutdown sender and consumer task handle. On
shutdown it:

1. stops API and proxy listeners as today;
2. awaits proxy listener tasks so no new producer path begins;
3. signals the consumer to drain events already accepted into the bounded
   channel, then stop; this does not wait for detached forwarding session
   publisher clones;
4. awaits the consumer task;
5. drops `EventPersistence` and joins the SQLite worker.

The receiver exposes a non-blocking drain operation for this shutdown path.
After it stops, publishers held by detached sessions observe `Closed`, increment
the closed-event counter, log a warning, and continue byte forwarding. The task
does not solve active-session draining; that remains Issue 118.

## Compatibility

- Existing API state, WebSocket subscription contracts, storage contracts, and
  SQLite schema remain unchanged.
- Configuration is additive and uses defaults for existing TOML files.
- Slow-query threshold configuration and retention scheduling remain separate
  tasks; the consumer continues using the existing default classifier.

## Tests

- Config defaults, TOML parsing, zero-capacity validation, and both overload
  policy values.
- Capture crate counter behavior for full and closed channels.
- App consumer fan-out to ring buffer, live statistics, broadcaster, and
  SQLite persistence.
- Runtime shutdown drains receiver-accepted events before persistence closes.
- App producer overload paths do not return forwarding errors.
