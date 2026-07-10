# Apply configured slow-query threshold at runtime

## Goal

Make the configured `proxy.slow_threshold_ms` control the status assigned to
successful captured SQL events at runtime.

## Confirmed Facts

- `ProxyConfig::slow_threshold_ms` defaults to `500`, and `CONFIG.md` documents
  that successful events at or above the value are `slow`.
- The capture consumer currently calls `store_sql_events`, which constructs
  `SlowQueryClassifier::default()` for every event batch. As a result, a
  non-default runtime configuration is ignored.
- The classifier already defines the desired comparison: only `ok` events at
  or above the inclusive threshold become `slow`; all other statuses remain
  unchanged.

## Requirements

- Construct the runtime slow-query classifier from
  `SqlLensConfig.proxy.slow_threshold_ms`.
- Provide that classifier to the single capture consumer, so classification
  happens before ring-buffer storage, live-statistics updates, WebSocket
  broadcasts, and optional SQLite persistence.
- Preserve the 500ms default behavior for runtimes created without an explicit
  configuration.
- Keep the public configuration schema and the capture classifier contract
  unchanged.

## Acceptance Criteria

- [x] Runtime construction derives the capture classifier threshold from
  `proxy.slow_threshold_ms`.
- [x] A successful event at the configured threshold is stored and broadcast
  with status `slow`; a lower-duration event stays `ok`.
- [x] Default runtime construction continues to classify at 500ms.
- [x] App-runtime tests cover at least two non-default thresholds.

## Out of Scope

- Changing the `slow_threshold_ms` configuration format or validation rules.
- Changing classification for errors, unknown events, or events already marked
  slow.
- Adding slow-query filtering, alerts, or new API fields.
