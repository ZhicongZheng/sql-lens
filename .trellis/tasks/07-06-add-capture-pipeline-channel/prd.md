# Add capture pipeline channel

## Goal

Issue 018: add a bounded capture event channel that accepts `SqlEvent` values from proxy/protocol logic and hands them to future storage and broadcast consumers.

## User Value

SQL Lens must keep packet forwarding independent from storage and UI work. A bounded capture channel gives the runtime an explicit, observable handoff point before storage, WebSocket, and statistics exist.

## Background

- `sql-lens-core` owns the protocol-neutral `SqlEvent` model.
- `ARCHITECTURE.md` requires a capture event channel, non-blocking proxy behavior, explicit overload handling, and dropped-event counters.
- Storage and WebSocket consumers are future tasks.

## Requirements

- Add a `sql-lens-capture` crate for capture pipeline primitives.
- Add the crate to the Cargo workspace.
- Use `tokio::sync::mpsc` for a bounded channel.
- Accept `sql_lens_core::SqlEvent` without protocol-specific assumptions.
- Make channel capacity configurable through a capture pipeline config type.
- Make overload policy explicit.
- Provide at least:
  - drop-newest behavior for full channels.
  - reject-new behavior for full channels.
- Track a dropped-event counter for overload drops/rejections.
- Keep publish path non-blocking by using `try_send`.
- Expose a receiver for the future storage/broadcast fan-out task.
- Add unit tests for enqueue/receive, drop-newest overload, reject-new overload, and closed receiver behavior.

## Out Of Scope

- Storage writer.
- WebSocket broadcaster.
- Protocol parsing.
- Runtime wiring in `sql-lens-app`.
- Persisted metrics.
- Prometheus/OpenTelemetry exporters.
- Redaction.
- Backpressure that blocks packet forwarding.
- Multi-consumer fan-out implementation.

## Acceptance Criteria

- [ ] `sql-lens-capture` exists as a workspace crate.
- [ ] Capture channel accepts `SqlEvent`.
- [ ] Channel capacity is configurable.
- [ ] Overload policy is explicit.
- [ ] Full channel with drop-newest policy drops the incoming event and increments the dropped counter.
- [ ] Full channel with reject-new policy returns the event and increments the dropped counter.
- [ ] Closed receiver produces a structured publish error.
- [ ] Publish path does not await.
- [ ] Unit tests cover enqueue/receive.
- [ ] Unit tests cover drop-newest overload.
- [ ] Unit tests cover reject-new overload.
- [ ] Unit tests cover closed receiver.
- [ ] `cargo fmt --check` passes.
- [ ] `cargo check --workspace` passes.
- [ ] `cargo test --workspace` passes.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes.

## Open Questions

None blocking.
