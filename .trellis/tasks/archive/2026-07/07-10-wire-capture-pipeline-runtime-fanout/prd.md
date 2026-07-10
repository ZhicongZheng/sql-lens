# Wire capture pipeline into app runtime fan-out

## Goal

Move SQL event fan-out out of the MySQL packet-forwarding loop by routing
protocol-emitted events through one bounded capture pipeline and consumer task.

## Confirmed Facts

- `sql-lens-capture` already provides a bounded `CapturePipeline`, a
  non-blocking publisher, a single-consumer receiver, explicit overload
  policies, and dropped-event counters.
- The app currently receives adapter events into a local vector and awaits
  direct classification, WebSocket broadcast, statistics, ring-buffer append,
  and SQLite persistence from the forwarding loop.
- Architecture requires one capture pipeline shared across sessions, with
  forwarding independent from UI/storage work and dropped events counted.
- Existing startup configuration has `capture_mode`, but no capacity or
  overload-policy setting. Storage capacity must not be reused for the pipeline
  because the values have different memory and retention semantics.
- This work must preserve the existing API, WebSocket, storage, SQLite worker,
  and protocol adapter contracts. Configured slow-query thresholds and
  retention are separate Issues 116 and 117.

## Requirements

- Create one runtime-owned capture pipeline shared by all configured proxy
  targets.
- Publish adapter-emitted events with `try_send` semantics and do not await the
  capture consumer from packet forwarding.
- Run one consumer task that classifies accepted events once and fans them out
  to the existing ring buffer, live statistics, WebSocket broadcaster, and
  optional SQLite persistence path.
- Define observable behavior for a full and closed capture channel without
  failing byte forwarding.
- Shut down the consumer cleanly before closing the SQLite persistence worker,
  so accepted events are drained during normal runtime shutdown.
- Add a `[capture]` configuration section with `capacity` and
  `overload_policy`; defaults are `1024` and `drop_newest`.
- Add focused tests for fan-out, full-channel behavior, closed-consumer
  behavior, and a shared pipeline across multiple proxy targets.

## Acceptance Criteria

- [x] Protocol event publication does not await storage, WebSocket, SQLite, or
      capture-consumer work.
- [x] A single capture consumer delivers each accepted event once to existing
      storage, statistics, WebSocket, and configured SQLite persistence.
- [x] Full-channel and closed-consumer outcomes are counted and logged without
      stopping proxy byte forwarding.
- [x] Runtime shutdown signals the consumer to drain already-accepted events,
      then stops SQLite persistence without losing those events or waiting for
      detached session publishers.
- [x] Existing ring-buffer-only and SQLite runtime tests remain valid.
- [x] Focused app-runtime tests cover pipeline fan-out and overload behavior.
- [x] Config parsing and validation cover capture pipeline defaults, invalid
      zero capacity, and both overload policies.
- [x] `rtk cargo fmt --check`, `rtk cargo test -p sql-lens-app --lib`,
      `rtk cargo test --workspace`, and
      `rtk cargo clippy --workspace --all-targets -- -D warnings` pass.

## Out Of Scope

- Configured slow-query threshold wiring, retention scheduling, plugins, TLS,
  or session draining.
- REST or WebSocket schema changes.
- Multiple capture consumers, per-subscriber queues, or a storage rewrite.

## Decision

- Add a public `[capture]` configuration section now. Its defaults are
  `capacity = 1024` and `overload_policy = "drop_newest"`; storage retention
  capacity remains an independent setting.
