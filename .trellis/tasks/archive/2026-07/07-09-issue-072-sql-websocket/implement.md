# Implement — Issue 072: SQL WebSocket client

## 1. WebSocket URL helper

- [ ] Create `src/lib/websocket/url.ts`:
      - `wsUrl(path: string): string` — derives WS URL from `apiBaseUrl`.
      - Replace `http:` → `ws:`, `https:` → `wss:`, append path.

## 2. WebSocket client module

- [ ] Create `src/lib/websocket/sql-stream.ts`:
      - Module-level `ws`, `state`, `backoff`, `reconnectTimer`.
      - `connect()`: creates WebSocket, sends subscribe message on open.
      - `disconnect()`: closes WS, clears reconnect timer.
      - `setOnEvent(callback)`: sets the event callback.
      - `getConnectionState()`: returns current state.
      - Reconnect with exponential backoff (1s→2s→4s→...→30s max).
      - Parse `sql_event.created` and `subscription.error` messages.

## 3. React hook

- [ ] Create `src/lib/websocket/use-sql-stream.ts`:
      - `useSqlStream()` returns `{ connectionState, latestEvent }`.
      - On mount: calls `connect()` if not already open.
      - Subscribes to state change + event callbacks.
      - On event: updates `latestEvent` + prepends to TanStack Query cache
        via `queryClient.setQueryData`.
      - Does NOT disconnect on unmount (global connection).

## 4. Export barrel

- [ ] Update `src/lib/websocket/index.ts` to export `useSqlStream`,
      `ConnectionState` type.

## 5. SQL List integration

- [ ] In `sql-events.tsx`:
      - Import `useSqlStream`.
      - Add live indicator in header: green/amber/red dot + label.
      - Add disconnect banner below filter bar when `connectionState === "closed"`.
      - Live events appear via TanStack Query cache update (no extra state).

## 6. Query client export

- [ ] Ensure `queryClient` is importable from `@/lib/query-client` (already
      exported as named export). No change needed if already done.

## Validation gates

- [ ] `npm run build` → exit 0.
- [ ] `npm run typecheck` → exit 0.
- [ ] `npm run test` → exit 0.
- [ ] `grep -rn "new WebSocket(" src/` → only in `sql-stream.ts`.
- [ ] No hardcoded status colors in sql-events.tsx.
- [ ] Connection state indicator uses `text-status-*` tokens.

## Rollback

`git checkout -- crates/sql-lens-app/web/src/lib/websocket/` reverts the new
modules. `git checkout -- crates/sql-lens-app/web/src/app/routes/sql-events.tsx`
reverts the integration.
