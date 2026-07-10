# Wire connection lifecycle into app runtime

## Goal

Make the connections API and live `active_connections` statistic reflect real
MySQL proxy sessions by recording their lifecycle in the app runtime.

## Confirmed Facts

- `sql-lens-proxy` already provides `ConnectionLifecycleRecord` with terminal
  close and failure transitions, including byte counts and timestamps.
- `sql-lens-storage` already provides bounded `ConnectionStore::upsert` and
  `LiveStatistics::{record_connection_opened, record_connection_closed}`.
- The runtime creates a `ConnectionInfo` after a backend dial succeeds, but
  currently only passes it to the protocol adapter. It never updates either
  store, so `GET /api/v1/connections` has no runtime data and active connection
  counts stay at zero.
- The public REST response schema and storage data types already represent the
  required lifecycle states. This task must not change those contracts.

## Requirements

- Record a successfully backend-connected proxy session in `ConnectionStore`
  and mark it active in live statistics before forwarding begins.
- Retain backend dial failures as terminal `failed` connection-history records,
  but never count them as active connections.
- Update the stored connection with final state, close timestamp, and byte
  counters when forwarding ends normally or fails.
- Remove the session from live active-connection statistics on every terminal
  forwarding path, without removing its final record from `ConnectionStore`.
- Preserve the proxy's non-blocking forwarding behavior; lifecycle bookkeeping
  must be bounded in-memory work and must not alter byte forwarding semantics.
- Keep the scope to runtime wiring and focused tests. Do not add persistence,
  API schema changes, protocol-state mirroring, or frontend changes.

## Acceptance Criteria

- [x] A successful proxied session appears in the runtime connection store with
      its ID, target identity, and a non-terminal state while it is active.
- [x] A normally closed session is retained as `closed`, with final byte counts
      and `closed_at` populated.
- [x] A failed forwarding session is retained as `failed`, with available byte
      counts and `closed_at` populated.
- [x] `active_connections` increases after a successful backend dial and
      returns to its prior value when the same session reaches a terminal
      forwarding outcome.
- [x] Backend dial failures are retained as terminal `failed` connection
      records, with `closed_at` populated, and do not create active sessions.
- [x] Focused app-runtime tests cover normal close, forwarding failure, and
      backend-dial failure behavior.
- [x] `rtk cargo fmt --check` and the focused app tests pass.

## Out Of Scope

- Protocol-level lifecycle state mirroring such as `handshake_seen`,
  `authenticating`, and `command_in_flight`.
- Persisting connection history to SQLite.
- Changes to REST or WebSocket schemas.
- Redaction, capture-pipeline, plugin, or frontend work.

## Decision

- Backend dial failures are visible in connection history as `failed` records.
  This preserves diagnostics for unreachable or misconfigured targets without
  inflating `active_connections`.
