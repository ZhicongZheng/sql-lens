# Apply Redaction Before Storage - Implementation Plan

## Checklist

- [x] Add `crates/sql-lens-core/src/redaction.rs`.
- [x] Define `DEFAULT_REDACTION_MASK` and documented default sensitive
      parameter names in core.
- [x] Define `RedactionPolicy` with default enabled behavior.
- [x] Implement `redact_sql_event(event, policy) -> SqlEvent`.
- [x] Implement helper logic for case-insensitive parameter-name matching.
- [x] Implement helper logic for literal SQL text pattern replacement.
- [x] Implement helper logic for redacted parameter value replacement in
      `expanded_sql`, `original_sql`, and `normalized_sql`.
- [x] Re-export redaction types/functions from `sql-lens-core/src/lib.rs`.
- [x] Add focused `sql-lens-core` tests for:
      - disabled policy leaves events unchanged,
      - sensitive parameter names match case-insensitively,
      - already-redacted parameters are masked,
      - SQL patterns apply to all SQL text fields,
      - expanded SQL parameter values are removed.
- [x] Update `RedactionConfig::default()` so documented sensitive names match
      `SECURITY.md`.
- [x] Add or update `sql-lens-config` tests for the redaction defaults.
- [x] Add `RedactionPolicy` storage to `RingBufferStore`.
- [x] Add `RingBufferStore::with_redaction_policy`.
- [x] Redact inside `RingBufferStore::append` before retention.
- [x] Add storage tests proving retained events contain masked parameters and
      masked expanded SQL.
- [x] Add `RedactionPolicy` storage to `SqlEventBroadcaster`.
- [x] Add `SqlEventBroadcaster::with_redaction_policy`.
- [x] Redact inside `SqlEventBroadcaster::publish` before broadcast.
- [x] Add broadcaster or WebSocket tests proving subscribers receive masked SQL
      text.
- [x] Update `.trellis/spec/backend/quality-guidelines.md` with the redaction
      contract.
- [x] Run targeted validation.
- [x] Run workspace validation.

## Validation Commands

Use JetBrains build/test tools first if the SQL Lens project is open in the
IDE. If not, use:

```bash
rtk cargo fmt --check
rtk cargo test -p sql-lens-core
rtk cargo test -p sql-lens-config
rtk cargo test -p sql-lens-storage
rtk cargo test -p sql-lens-api
rtk cargo test --workspace
rtk cargo clippy --workspace --all-targets -- -D warnings
```

Also scan the touched backend source for temporary output:

```bash
rtk rg -n "tracing::|println!|eprintln!|dbg!" crates/sql-lens-core/src crates/sql-lens-storage/src crates/sql-lens-api/src crates/sql-lens-config/src
```

## Risky Files

- `crates/sql-lens-core/src/event.rs`: public event model. Avoid changing
  existing field names or enum variants in this task.
- `crates/sql-lens-storage/src/ring_buffer.rs`: storage append and query tests
  rely on exact retained events.
- `crates/sql-lens-api/src/live_sql_events.rs`: broadcast behavior affects
  WebSocket tests.
- `crates/sql-lens-api/src/websocket.rs`: subscription semantics must remain
  "valid subscribe required before events".
- `crates/sql-lens-config/src/model.rs`: config defaults are a public contract.

## Review Gates

- [x] No raw secret appears in retained storage test events.
- [x] No raw secret appears in WebSocket/broadcaster test payloads.
- [x] Existing API response schemas remain unchanged.
- [x] No new dependencies were added.
- [x] `00-bootstrap-guidelines` remains active and is not archived.

## Rollback Points

- If core redaction logic is wrong, fix in core before changing storage/API
  call sites.
- If storage tests regress broadly, temporarily remove storage boundary wiring
  while keeping the core redactor tests.
- If WebSocket tests regress, verify that filtering still happens after
  broadcast subscription and that redaction does not modify filter fields.
