# Add WebSocket filters

## Goal

Implement Issue 036: let `/ws/sql` clients subscribe to filtered SQL event streams so live dashboard clients can receive only relevant events.

## Background

- Issue 035 is complete: `/ws/sql` requires a valid `subscribe` message before sending `sql_event.created` live events.
- `API.md` already documents a `filters` object in the subscribe message.
- `ARCHITECTURE.md` says the WebSocket layer applies filters before sending matching events.
- REST SQL event listing already maps query parameters into `SqlEventFilter`.
- `SqlEventFilter` already supports matching by protocol, database, status, and duration range.
- Issue 036 acceptance requires invalid filters to return a subscription error.
- Product decision: subscription errors use the standard WebSocket envelope shape with `type`, `version`, and `payload`.

## Requirements

- Parse optional `filters` from the WebSocket `subscribe` message.
- Supported filters:
  - `protocol`: exact protocol name, for example `"mysql"`,
  - `status`: one or more of `"ok"`, `"slow"`, `"error"`, `"unknown"`,
  - `database`: exact database name,
  - `min_duration_ms`: inclusive minimum duration,
  - `max_duration_ms`: inclusive maximum duration.
- Forward only matching `sql_event.created` messages to that subscriber.
- Subscribers without filters continue receiving all future SQL events.
- Invalid filters must return a WebSocket subscription error instead of silently subscribing.
- Subscription error messages use:
  - `type: "subscription.error"`,
  - `version: 1`,
  - `payload.code: "INVALID_FILTER"`,
  - `payload.message`,
  - `payload.field`.
- After a subscription error, the socket remains open and continues waiting for a valid `subscribe` message.
- Keep filters protocol-neutral.
- Do not implement historical replay, auth, frontend code, persistent saved filters, or non-required filter fields in this task.

## Acceptance Criteria

- [x] Subscribe with `protocol` filter sends matching events and suppresses non-matching events.
- [x] Subscribe with `status` filter supports multiple statuses.
- [x] Subscribe with `database` filter sends matching events and suppresses non-matching events.
- [x] Subscribe with duration range filters sends matching events and suppresses non-matching events.
- [x] Subscribe without filters still receives all events.
- [x] Invalid filters return a subscription error message.
- [x] Subscription errors use `type: "subscription.error"`, `version: 1`, and `payload.code: "INVALID_FILTER"`.
- [x] After an invalid filter error, the same socket can still subscribe later with valid filters.
- [x] Tests cover matching and non-matching events.
- [x] Existing SQL WebSocket subscription tests still pass.
- [x] `cargo fmt --check` passes.
- [x] `cargo test -p sql-lens-api` passes.
- [x] `cargo test --workspace` passes.
- [x] `cargo clippy --workspace --all-targets -- -D warnings` passes.

## Out of Scope

- Historical replay of already stored events.
- URL query parameter filters for WebSocket upgrade.
- Text search, fingerprint, user, client address, or timestamp filters.
- Filter persistence.
- RBAC/auth-aware filters.
- Frontend filter UI.
