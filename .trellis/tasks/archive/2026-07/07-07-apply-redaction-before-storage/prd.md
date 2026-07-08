# Apply redaction before storage

## Goal

Implement Issue 056 by ensuring captured SQL events are sanitized before they
can be retained by storage or delivered through the live SQL WebSocket stream.

The first implementation should keep redaction protocol-neutral, deterministic,
and cheap enough for local developer usage. It should protect structured
parameters and SQL text fields without introducing a full SQL parser, regex
engine, plugin rule system, or persistent migration layer.

## Source Issue

Issue 056: Apply redaction before storage.

Description: Ensure sensitive parameters and SQL text are redacted before
events reach storage or WebSocket.

Labels: `area:security`, `area:storage`, `type:feature`
Priority: P0
Difficulty: Hard
Dependencies: Issue 055

## Confirmed Facts

- `SqlEvent` already contains `original_sql`, `normalized_sql`, `expanded_sql`,
  and `parameters` fields in `crates/sql-lens-core/src/event.rs`.
- `SqlParameter` already contains `value` and `redacted` fields, so the shared
  event contract has an existing redaction marker.
- `RedactionConfig` already exists in `crates/sql-lens-config/src/model.rs`
  with `enabled`, `mask`, `parameter_names`, and `sql_patterns` fields.
- `SECURITY.md` lists storage write, WebSocket broadcast, API serialization,
  exporters, parameter decoding, and SQL expansion as redaction points.
- `SECURITY.md` lists default sensitive names: `password`, `passwd`, `token`,
  `secret`, `api_key`, `access_key`, and `refresh_token`.
- `RingBufferStore::append` currently stores the incoming `SqlEvent` as-is.
- `SqlEventBroadcaster::publish` currently sends the incoming `SqlEvent` as-is.
- WebSocket `sql_event.created` messages are built from
  `SqlEventSummaryResponse::from(&SqlEvent)`, so broadcast-side redaction must
  happen before subscription delivery.
- Issue 055 intentionally kept MySQL expanded SQL local until this redaction
  task defines exposure rules.

## Requirements

- R1. Provide a shared, protocol-neutral redaction policy and event redaction
  function in `sql-lens-core` so storage, API, and future exporters can reuse
  one implementation.
- R2. Keep the policy dependency-free: no regex crate, SQL parser, classifier,
  or plugin integration in this issue.
- R3. Default redaction must be enabled, use the mask `***`, and include the
  sensitive parameter names documented in `SECURITY.md`.
- R4. Parameter-name matching must be case-insensitive exact matching.
- R5. When a parameter matches the policy, set `redacted = true` and replace
  its value with the configured mask.
- R6. Parameters that arrive with `redacted = true` must stay redacted and must
  not retain their original value after sink-boundary redaction.
- R7. SQL text patterns must be applied to `original_sql`, `normalized_sql`,
  and `expanded_sql` using literal substring replacement.
- R8. Sensitive structured parameter values must also be removed from SQL text
  fields, including `expanded_sql`, so prepared statement expansion cannot leak
  a parameter value that has already been redacted.
- R9. Empty pattern strings and empty sensitive values must be ignored to avoid
  accidental whole-string replacement.
- R10. `RingBufferStore::append` must redact events before retention. Existing
  read APIs should return only the retained, redacted event.
- R11. `SqlEventBroadcaster::publish` must redact events before sending them to
  subscribers. Existing WebSocket filters must continue to work.
- R12. The implementation must not log raw SQL, raw parameter values, auth
  payloads, or database error text.
- R13. Existing public API response shapes must remain compatible; do not add a
  new `SqlParameterValue` variant in this issue.

## Acceptance Criteria

- [x] Core redaction tests prove case-insensitive parameter-name matching,
      already-redacted parameter handling, SQL pattern replacement, and
      expanded SQL value masking.
- [x] Config default tests prove redaction defaults match the sensitive names
      documented in `SECURITY.md`.
- [x] Storage tests prove a sensitive parameter and matching expanded SQL value
      are redacted after `RingBufferStore::append`.
- [x] WebSocket or broadcaster tests prove subscribers receive redacted SQL
      text rather than raw sensitive values.
- [x] Existing storage timeline/detail behavior remains compatible with
      redacted events.
- [x] Existing WebSocket subscription behavior, including required subscribe
      handling and filters, remains compatible.
- [x] No new runtime dependencies are added.
- [x] `rtk cargo fmt --check` passes.
- [x] `rtk cargo test -p sql-lens-core` passes.
- [x] `rtk cargo test -p sql-lens-storage` passes.
- [x] `rtk cargo test -p sql-lens-api` passes.
- [x] `rtk cargo test --workspace` passes.
- [x] `rtk cargo clippy --workspace --all-targets -- -D warnings` passes.

## Out Of Scope

- Regex-based SQL redaction.
- SQL parsing, AST-level rewriting, or dialect-specific redaction.
- Column-name heuristics beyond structured parameter names.
- Value classifiers for credit cards, emails, phone numbers, or PII.
- Plugin-provided redaction rules.
- Persistent storage migrations for previously retained unredacted events.
- UI changes.
- Replay behavior.
- Runtime config hot reload integration.
- MySQL protocol changes or prepared statement event emission changes.

## Open Questions

None blocking. The recommended scope is to implement a conservative shared
redaction policy now, then wire richer config/hot-reload behavior in a later
task when the application composition layer owns runtime configuration flow.

## Notes

- This task is intentionally sink-boundary first. Future capture fan-out can
  redact once before cloning to storage, WebSocket, exporters, and statistics,
  but the current codebase does not yet have that central fan-out layer.
