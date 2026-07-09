# Issue 072: Add SQL WebSocket client

## Goal

Connect the frontend to the backend's `/ws/sql` WebSocket endpoint so new
SQL events appear in the SQL List in real time, replacing the need for manual
page refresh or polling. This establishes the live-update foundation that
the pause/resume control (Issue 073) and future features depend on.

## Requirements

### R1 — WebSocket client module

Create `src/lib/websocket/sql-stream.ts` implementing the `/ws/sql` client.

- Reads `apiBaseUrl` from `@/lib/api/config` and derives the WebSocket URL
  (same origin, `ws://` or `wss://` depending on page protocol, path `/ws/sql`).
- On open: sends a `subscribe` message (version 1, no filters — filters are
  a follow-up once the filter bar wiring is connected).
- On message: parses JSON, dispatches `sql_event.created` events to a
  callback, ignores `subscription.error` (logs to console.warn).
- On close: automatic reconnect with exponential backoff (1s, 2s, 4s, max
  30s). Reset backoff on successful open.
- Exposes connection state: `"connecting" | "open" | "closed" | "reconnecting"`.

### R2 — `useSqlStream` hook

Create `src/lib/websocket/use-sql-stream.ts` exporting a `useSqlStream()` hook.

- Manages a singleton WebSocket connection (shared across components via a
  module-level ref, not a React context — the stream is global).
- Returns `{ connectionState, latestEvent }`:
  - `connectionState`: the current WebSocket state.
  - `latestEvent`: the most recent `SqlEventSummary` received (or `null`).
- On each new `sql_event.created` event, updates `latestEvent` AND prepends
  the event into the TanStack Query cache for `["sql-events"]` via
  `queryClient.setQueryData`. This makes the SQL List table update live
  without a refetch.
- Only one active connection at a time (the hook mounts/unmounts without
  closing the shared connection; only the module-level cleanup closes it).

### R3 — SQL List live prepend

Update `sql-events.tsx` to consume `useSqlStream()`:

- When `latestEvent` arrives, the event appears at the top of the SQL List
  (via the TanStack Query cache update in R2).
- The table does NOT auto-scroll (the user may be inspecting paused data;
  auto-scroll is Issue 073).
- A live indicator in the page header shows connection state:
  - `open`: green dot + "Live" (`text-status-ok`).
  - `connecting`/`reconnecting`: amber dot + "Connecting…" (`text-status-slow`).
  - `closed`: red dot + "Disconnected" (`text-status-error`).

### R4 — Disconnect visibility

When the WebSocket is disconnected (`connectionState === "closed"`), show a
non-intrusive banner below the filter bar: "Live updates disconnected.
Reconnecting…" with a muted style. The banner disappears when connection is
restored.

## Acceptance Criteria

- [ ] `npm run build` exits 0.
- [ ] `npm run typecheck` exits 0.
- [ ] `npm run test` exits 0.
- [ ] WebSocket client connects to `/ws/sql` and sends a valid `subscribe`
      message.
- [ ] `sql_event.created` events update the TanStack Query cache (SQL List
      shows new events without manual refresh).
- [ ] Connection state indicator in the SQL List header shows live/connecting/
      disconnected with correct `text-status-*` tokens.
- [ ] Disconnect banner appears when connection is lost.
- [ ] Automatic reconnect with exponential backoff works.
- [ ] No `new WebSocket(` calls outside `src/lib/websocket/` (grep
      assertion).
- [ ] No hardcoded status colors.
- [ ] Dark mode renders correctly.

## Out of Scope

- Pause/resume live updates (Issue 073).
- WebSocket filter subscription from the filter bar (follow-up).
- Statistics WebSocket stream (`/ws/statistics`) (future issue).
- Connection status in the topbar (future enhancement).

## Constraints

- WebSocket URL derived from `apiBaseUrl` (not hardcoded).
- No new dependencies (WebSocket is a browser native API).
- TanStack Query cache mutation uses `queryClient.setQueryData` — no
  separate state store for live events.
