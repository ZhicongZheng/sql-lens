# Design — Issue 072: SQL WebSocket client

## WebSocket URL derivation

From `apiBaseUrl` (e.g. `http://127.0.0.1:5173`):
- Replace `http:` → `ws:`, `https:` → `wss:`.
- Append `/ws/sql`.
- No port override — uses the same port as the REST API.

Helper: `function wsUrl(path: string): string` in `src/lib/websocket/url.ts`.

## Connection architecture

Module-level singleton (not React context):

```
src/lib/websocket/sql-stream.ts
  - ws: WebSocket | null
  - state: ConnectionState
  - backoff: number (1s → 2s → 4s → ... → 30s max)
  - connect(): void       — opens WS, sends subscribe, sets up handlers
  - disconnect(): void    — closes WS, stops reconnect
  - onEvent: (ev) => void — callback set by the hook
  - getState(): ConnectionState
```

The module exports `connect()`, `disconnect()`, `onEvent` setter, and
`getState()`. React integration is via `useSqlStream()` hook.

## Reconnect strategy

- On close (unexpected): wait `backoff` ms, then `connect()`.
- Backoff: starts at 1s, doubles on each failure, caps at 30s.
- On successful open: reset backoff to 1s.
- On intentional `disconnect()`: no reconnect.
- `connectionState` transitions:
  - `closed` → `connecting` (on connect call)
  - `connecting` → `open` (on WS open event)
  - `open` → `reconnecting` (on unexpected close)
  - `reconnecting` → `connecting` (after backoff, before reconnect)
  - Any → `closed` (on intentional disconnect)

## TanStack Query cache integration

On each `sql_event.created` event:

1. Parse the payload into a partial `SqlEventSummary`.
2. Use `queryClient.setQueryData(["sql-events", undefined], updater)` to
   prepend the event to the first page.
3. The updater function receives the current `PaginatedResponse<SqlEvent>`
   and returns it with the new event prepended to `items`.
4. The event payload from WebSocket is a summary (not a full SqlEvent), so
   we create a stub `SqlEvent` with the summary fields and empty defaults
   for fields not in the stream (e.g. `expanded_sql`, `metadata`). The full
   detail is fetched on-demand when the user clicks the row (future SQL
   Detail).

## Hook: `useSqlStream()`

```ts
export function useSqlStream(): {
  connectionState: ConnectionState;
  latestEvent: SqlEventSummary | null;
}
```

- On mount: calls `connect()` if not already connected.
- Subscribes to state changes and event callbacks.
- Does NOT disconnect on unmount (the connection is global/shared).
- Returns the latest event and current state.

## File changes

| File | Change |
|---|---|
| `src/lib/websocket/url.ts` | **new** — WS URL helper |
| `src/lib/websocket/sql-stream.ts` | **new** — singleton WS client |
| `src/lib/websocket/use-sql-stream.ts` | **new** — React hook |
| `src/lib/websocket/index.ts` | update barrel |
| `src/app/routes/sql-events.tsx` | add live indicator + disconnect banner |
| `src/lib/query-client.ts` | export queryClient for cache mutation |

## Non-goals

- No pause/resume (073). No WS filters. No statistics WS stream.
- No auto-scroll on new events.
