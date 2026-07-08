# Issue 082: Add slow SQL classification

## Goal

Classify successful SQL events as `slow` when their captured duration is at or
above a configurable global threshold. The classification should happen after
protocol adapters emit normalized events and before those events reach storage,
live statistics, WebSocket broadcast, or API-visible state.

This keeps protocol adapters focused on wire observation while making the
existing `CaptureStatus::Slow`, SQL event status filters, and statistics
`slow_count` useful for real captured traffic.

## Source Issue

Issue 082: Add slow SQL classification.

- Description: Classify SQL events as slow based on configured thresholds.
- Acceptance: global threshold is supported; slow status appears in stored
  events; tests cover below and above threshold.
- Labels: `area:capture`, `area:statistics`, `type:feature`
- Priority: P0
- Dependencies: Issue 003, Issue 007

## Requirements

- R1. Add a protocol-neutral slow SQL classifier for `SqlEvent` values.
- R2. The classifier must convert `CaptureStatus::Ok` to
  `CaptureStatus::Slow` when `event.duration >= threshold`.
- R3. The classifier must not change `Error`, `Unknown`, or already `Slow`
  statuses.
- R4. The threshold must be globally configurable in the config model with a
  sensible default.
- R5. The app capture fan-out used by the minimal MySQL runtime must classify
  events before storage and broadcast so REST API responses can show `slow`.
- R6. Live statistics must count classified slow events when the app fan-out
  records captured events.
- R7. Keep MySQL protocol parsing unchanged; protocol adapters should continue
  emitting terminal `ok`/`error` status based on backend packets.
- R8. Keep frontend and UI work out of scope.

## Acceptance Criteria

- [x] A global slow threshold config field exists with a default.
- [x] Unit tests prove below-threshold successful events remain `ok`.
- [x] Unit tests prove at-threshold and above-threshold successful events become
      `slow`.
- [x] Unit tests prove error/unknown/already-slow events are not overwritten.
- [x] Stored/API-visible events from the app fan-out can be `slow`.
- [x] Live statistics `slow_count` increments for classified slow events.
- [x] Existing SQL event status parsing/serialization remains compatible with
      `ok`, `slow`, `error`, and `unknown`.
- [x] `rtk cargo fmt --check` passes.
- [x] `rtk cargo test --workspace` passes.
- [x] `rtk cargo clippy --workspace --all-targets -- -D warnings` passes.

## Notes

- Existing core model already has `CaptureStatus::Slow`.
- Existing storage statistics already count `CaptureStatus::Slow`; this task
  wires classification into the capture fan-out path instead of changing API
  response schemas.
