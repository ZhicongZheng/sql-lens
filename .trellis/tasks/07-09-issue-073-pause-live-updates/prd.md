# Issue 073: Add pause live updates control

## Goal

Let users pause live SQL List updates so they can inspect events without the
table jumping. While paused, incoming WebSocket events are queued; on resume
they flush into the table. This is essential for a debugging tool where the
user needs to read and click rows without interruption.

## Requirements

### R1 — Pause/resume toggle

- A toggle button in the SQL List page header (next to the live indicator).
  - **Playing** (default): `Pause` icon + "Live" label. Clicking pauses.
  - **Paused**: `Play` icon + "Paused" label + queued event count badge.
    Clicking resumes.
- Use shadcn `Button variant=ghost size=icon` with lucide `Pause` / `Play`
  icons.
- `aria-label="Pause live updates"` / `"Resume live updates"`.

### R2 — Event queuing while paused

- Modify `useSqlStream()` to accept an options object: `{ paused?: boolean }`.
- When `paused` is true: incoming `sql_event.created` events are accumulated
  in a module-level queue (array) instead of being flushed to the TanStack
  Query cache.
- When `paused` changes to false (resume): all queued events are flushed
  into the cache at once (prepended to the existing list, deduped by ID),
  and the queue is cleared.
- The queue has a soft cap of 200 events (oldest dropped first) to prevent
  unbounded memory growth.

### R3 — Visible state

- The header live indicator shows "Paused" (amber color) instead of "Live"
  (green) when paused.
- A `Badge variant="secondary"` shows the queued event count (e.g. "12
  queued"). This is hidden when count is 0 or when not paused.
- The live stream dot continues to reflect WebSocket connection state
  (independent of pause state).

### R4 — Persist pause state

- Pause state is NOT persisted to localStorage or URL (it's a transient
  debugging state). Refreshing the page resumes live updates.

## Acceptance Criteria

- [ ] `npm run build` exits 0.
- [ ] `npm run typecheck` exits 0.
- [ ] `npm run test` exits 0.
- [ ] Pause button appears in SQL List header and toggles pause/resume.
- [ ] While paused, incoming events are queued (not shown in table).
- [ ] Queued event count badge shows while paused.
- [ ] On resume, queued events flush to the table.
- [ ] Queue caps at 200 (oldest dropped).
- [ ] Live indicator shows "Paused" (amber) when paused, independent of
      WebSocket connection state.
- [ ] No `new WebSocket(` outside `sql-stream.ts`.
- [ ] Dark mode renders correctly.

## Out of Scope

- Persisting pause state across refresh.
- Per-filter pause (pause only some streams).
- Statistics stream pause.

## Constraints

- Use existing `useSqlStream` hook — extend, don't replace.
- No new dependencies.
