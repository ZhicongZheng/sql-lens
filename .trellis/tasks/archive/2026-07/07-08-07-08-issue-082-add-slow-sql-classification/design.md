# Design — Issue 082: Slow SQL Classification

## Boundary

Slow SQL classification is capture enrichment, not protocol parsing. MySQL and
future protocol adapters should continue to emit `ok` or `error` from wire
terminal packets. A protocol-neutral classifier then applies policy based on
event duration before events are retained or broadcast.

## Proposed Shape

- Add a small classifier in `sql-lens-capture`.
- Add `slow_threshold_ms` to `ProxyConfig` for now because the existing config
  model keeps capture mode under `[proxy]` and the runtime is still proxy-first.
- Use a default threshold of `500` ms.
- In `sql-lens-app::store_sql_events`, classify each event before publishing and
  appending.
- Record the classified event into `ApiState` live statistics in the same loop.

## Contracts

- `Ok` + duration below threshold → remains `Ok`.
- `Ok` + duration equal to threshold → becomes `Slow`.
- `Ok` + duration above threshold → becomes `Slow`.
- `Error`, `Unknown`, and `Slow` → unchanged.
- Threshold `0` means every successful event is slow. Config validation does not
  reject it because it is useful for tests and local debugging.

## Data Flow

```text
protocol adapter emits SqlEvent
  -> app capture fan-out classifies by duration
  -> WebSocket broadcaster receives classified event
  -> RingBufferStore receives classified event
  -> LiveStatistics records classified event
  -> REST API exposes classified status and slow_count
```

## Out Of Scope

- Per-route, per-user, per-database, or per-fingerprint thresholds.
- Frontend display changes.
- Historical recalculation of existing retained events when threshold changes.
- SQL fingerprinting or query normalization.
- Protocol-specific classification logic.

## Risk

`sql-lens-app` currently owns only a minimal test/runtime composition path, not a
full config-driven runtime. The first implementation can use the default
classifier there while making the config contract available for later startup
composition.
