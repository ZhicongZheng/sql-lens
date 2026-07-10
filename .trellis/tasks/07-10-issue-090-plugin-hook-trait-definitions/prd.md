# Issue 090: Add plugin hook trait definitions

## Goal

Define the smallest stable, protocol-neutral in-process plugin hook contract so
future exporters and classifiers can observe SQL Lens lifecycle events without
coupling plugin code to the proxy or MySQL adapter.

## Background

- `crates/sql-lens-plugin/src/lib.rs` currently contains only crate-level
  documentation and has no public contract types.
- `PLUGIN.md` defines five hook points: `OnConnect`, `OnQuery`, `OnPrepare`,
  `OnExecute`, and `OnError`.
- `sql-lens-core` already owns the shared `SqlEvent`, `ConnectionInfo`,
  `PreparedStatementInfo`, `SqlParameter`, `ErrorSummary`, and
  `ProtocolMetadata` models.
- The backend directory guidelines require plugin payloads to use stable,
  protocol-neutral models with optional metadata, and forbid blocking TCP
  forwarding on plugin hooks.
- Issue 091 (webhook exporter) depends on this contract.

## Requirements

- Add public plugin hook traits for connect, query, prepare, execute, and error
  events, matching the hook names and payload intent documented in `PLUGIN.md`.
- Use protocol-neutral core types in hook payloads; do not add MySQL-specific
  fields to plugin contracts.
- Define explicit result/error behavior for hook invocation so one plugin
  failure can be observed by the caller without becoming a proxy or capture
  failure.
- Keep the contract synchronous and dependency-light unless planning evidence
  requires otherwise; this task defines contracts only and does not wire hooks
  into runtime capture.
- Keep the public hook contracts owned and re-exported from the
  `sql-lens-plugin` crate root; shared payload data may reuse the stable,
  protocol-neutral models owned by `sql-lens-core`.
- Add unit tests that construct representative payloads and verify the public
  traits are object-safe where trait objects are needed by the future plugin
  registry.

## Acceptance Criteria

- [x] `sql-lens-plugin` exposes public hook contracts for `OnConnect`,
      `OnQuery`, `OnPrepare`, `OnExecute`, and `OnError`.
- [x] Hook payloads cover the documented connection, event, statement,
      parameter, expanded SQL, error, and protocol metadata data without
      leaking raw packet-specific types.
- [x] Hook traits are object-safe and can be held behind trait objects.
- [x] Hook failures have a typed, documented contract; this contract exposes
      failures without introducing a forwarding or capture control-flow path.
- [x] Unit tests cover payload construction, representative hook invocation,
      and object-safe trait usage.
- [x] `PLUGIN.md` and backend specs remain aligned with the implemented public
      contract.
- [x] No proxy, protocol, storage, API, frontend, exporter transport, or
      runtime wiring is added in this task.
- [x] `rtk cargo fmt --check` passes.
- [x] `rtk cargo test -p sql-lens-plugin` passes.
- [x] `rtk cargo test --workspace` passes.
- [x] `rtk cargo clippy --workspace --all-targets -- -D warnings` passes.

## Out Of Scope

- Loading plugins from disk or dynamically discovering implementations.
- Runtime hook dispatch from the capture pipeline or proxy forwarding path.
- Webhook, Prometheus, OpenTelemetry, or file exporter implementations.
- Plugin configuration changes, permissions, sandboxing, retries, or async
  queues.
- SQL rewriting, packet mutation, authentication, or frontend changes.

## Resolved Decisions

- Use five separate synchronous traits: `OnConnect`, `OnQuery`, `OnPrepare`,
  `OnExecute`, and `OnError`.
- Each trait returns the shared `PluginResult` type and receives borrowed
  protocol-neutral core models. This keeps the contract object-safe and avoids
  cloning event payloads during future dispatch.
- Keep runtime dispatch, redaction enforcement, plugin loading, and async
  scheduling out of this task. Future dispatchers must pass redacted event
  values to hooks according to the safety rules in `PLUGIN.md`.
