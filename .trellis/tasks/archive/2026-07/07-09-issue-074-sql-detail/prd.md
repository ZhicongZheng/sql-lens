# Issue 074: Build SQL Detail page

## Goal

Replace the detail drawer placeholder with a fully functional SQL event
detail view. When a user clicks a row in the SQL List, the right-side drawer
opens showing the full event data: SQL text, parameters, timings, result,
error, connection info, and protocol metadata. This completes the primary
debugging workflow: browse → click → inspect.

## Requirements

### R1 — Drawer context: selected event ID

Update `DetailDrawerProvider` to track the selected event ID:

- `selectedEventId: string | null`.
- `openDrawer(eventId: string)` — sets the ID and opens the drawer.
- `closeDrawer()` — clears the ID and closes the drawer.
- The `openDrawer` signature changes from `() => void` to
  `(eventId?: string) => void` (backwards compatible — calling without an ID
  still opens the drawer with no content).

### R2 — SQL Detail component

Create `src/components/sql/sql-detail.tsx` rendering the full event detail
inside the detail drawer.

Data source: `useSqlEvent(selectedEventId)` from `@/lib/api/hooks`. The hook
fetches `GET /api/v1/sql-events/{id}` and returns the full `SqlEvent`.

Sections (from UI.md), each in a labeled group:

| Section | Content |
|---|---|
| **Summary** | Status badge (`text-status-*`), protocol, database, user, duration, timestamp, rows (returned/affected) |
| **SQL** | Original SQL in a monospace `<pre>` block with a Copy button. Toggle to show expanded SQL when it differs from original. |
| **Parameters** | Table: index, name (if available), type, value, redaction state. Data from `metadata` (protocol-specific). For now show a "No parameters" placeholder if metadata is empty. |
| **Timings** | Duration displayed prominently. Connection ID as a clickable link (future: open connection detail). |
| **Result** | Rows returned/affected. |
| **Error** | If `status === "error"`, show error info (status badge + any error detail from metadata). Otherwise hidden. |
| **Connection** | `connection_id`, `client_addr`, `backend_addr`, `target_name`. |
| **Protocol metadata** | Raw JSON display of `metadata` field in a collapsible `<pre>` block. |
| **Replay** | A "Replay" button (shadcn Button) that triggers `usePreviewReplay()`. For now, just show a toast "Replay preview is not yet wired" — the actual replay flow is a follow-up. |

### R3 — Loading and error states

- **Loading**: skeleton blocks for each section.
- **Not found**: if `useSqlEvent(id)` returns an error with `NOT_FOUND` code,
  show "Event not found" with a muted message.
- **Error**: generic error state with AlertTriangle icon.

### R4 — Copy actions

- Copy button next to SQL text (original and expanded). Uses
  `navigator.clipboard.writeText()`. On success, show a `toast("SQL copied")`.
- Copy button is a small `Button variant=ghost size=icon` with a `CopyIcon`.

### R5 — SQL List integration

Update `sql-events.tsx`:
- When a row is clicked, call `openDrawer(event.id)` (not just
  `openDrawer()`).
- The drawer content switches from the placeholder to `<SqlDetail />`.

### R6 — Detail drawer update

Update `detail-drawer.tsx`:
- When `selectedEventId` is set, render `<SqlDetail />` inside the Sheet.
- When no event is selected, show the existing placeholder text.
- Sheet title updates to show the event ID or "SQL Detail".

## Acceptance Criteria

- [ ] `npm run build` exits 0.
- [ ] `npm run typecheck` exits 0.
- [ ] `npm run test` exits 0.
- [ ] Clicking a SQL List row opens the detail drawer with full event data.
- [ ] Summary section shows status badge, protocol, database, user, duration,
      timestamp, rows.
- [ ] SQL section shows original SQL in monospace with a Copy button.
- [ ] Toggle shows expanded SQL when it differs from original.
- [ ] Parameters section shows parameter table or "No parameters" placeholder.
- [ ] Error section visible only when status is "error".
- [ ] Connection section shows connection_id, client/backend addr, target.
- [ ] Protocol metadata shown as collapsible raw JSON.
- [ ] Copy button copies SQL to clipboard and shows toast.
- [ ] Loading state shows skeleton blocks.
- [ ] Not found state shows "Event not found".
- [ ] No `fetch` calls in the component (data via hooks only).
- [ ] No hardcoded status colors.
- [ ] Dark mode renders correctly.

## Out of Scope

- Monaco Editor for SQL display (Issue 075).
- Replay execution flow (future — button shows placeholder toast).
- Parameter redaction toggle (future).
- Connection detail view (future).

## Constraints

- Use `useSqlEvent(id)` from `@/lib/api/hooks`.
- Use shadcn `Badge`, `Button`, `Skeleton`, `Separator` from 065 baseline.
- No new dependencies.
