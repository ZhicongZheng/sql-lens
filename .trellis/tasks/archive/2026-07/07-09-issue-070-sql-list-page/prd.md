# Issue 070: Build SQL List page

## Goal

Replace the route stub in `sql-events.tsx` with a functional SQL event
timeline table — the primary working surface for SQL Lens. This is the first
data-heavy table page and establishes the pattern for all subsequent list
views (Connections, Statistics detail, etc.).

## Requirements

### R1 — SQL event table

Display the columns from UI.md, in this order:

| # | Column | Source field | Format |
|---|---|---|---|
| 1 | Time | `timestamp` | `HH:MM:SS` (from ISO string) |
| 2 | Protocol | `protocol` | lowercase string |
| 3 | Database | `database` | string |
| 4 | User | `user` | string |
| 5 | Client | `client_addr` | string |
| 6 | Duration | `duration_ms` | `{n}ms` |
| 7 | Status | `status` | `Badge` with `text-status-*` token + word |
| 8 | Rows | `rows.returned` | number (show `returned`; `affected` in detail) |
| 9 | SQL preview | `original_sql` | monospace, truncated (`max-w-xs truncate`) |

Use the shadcn `Table` component from 065. Status column uses `Badge` with
the `text-status-ok/slow/error/unknown` tokens (never hardcoded colors,
always paired with the word).

### R2 — Data fetching

Use `useSqlEvents(params)` from `@/lib/api/hooks` with default params
(no filters for now — filters come in Issue 071). The hook returns
`PaginatedResponse<SqlEvent>` with `items` and `next_cursor`.

Implement cursor-based "Load more":

- Show the first page of results on mount.
- A "Load more" button at the table bottom calls `refetch()` with the
  `next_cursor` as the `cursor` param, appending new rows to the existing
  list.
- When `next_cursor` is absent/undefined, the button is hidden (all data
  loaded).
- Maintain a local `allItems` array that accumulates pages as they load.

### R3 — Loading state

While the initial fetch is in flight (`isLoading`), show a `Skeleton` row
placeholder for each of 9 columns, repeated 5 times, inside the table body.
The table header stays visible so the layout doesn't shift.

### R4 — Empty state

If the fetch returns successfully but `items` is empty, show a centered
message inside the table area: "No SQL events captured yet." in muted text.
The table header remains visible.

### R5 — Error state

If the fetch returns an error, show an error message inside the table area:
an `AlertTriangle` icon + "Failed to load SQL events" + `error.message`.
Use `text-status-error`. Table header remains visible.

### R6 — Row styling

- Slow rows (`status === "slow"`): the entire row gets a subtle left border
  accent (`border-l-2 border-status-slow`).
- Error rows (`status === "error"`): `border-l-2 border-status-error`.
- OK and unknown rows: no special border.
- All rows are clickable (cursor-pointer) — clicking opens the detail
  drawer via `useDetailDrawer().openDrawer()`. The event ID is stored in
  the drawer context for future SQL Detail content. For now the drawer
  shows its placeholder text.

### R7 — Responsive behavior

- Desktop: full 9-column table.
- Mobile (`< md`): the SQL preview column takes full width, other columns
  are hidden or condensed. The table scrolls horizontally if needed
  (`overflow-x-auto`).

## Acceptance Criteria

- [ ] `npm run build` exits 0.
- [ ] `npm run typecheck` exits 0.
- [ ] `npm run test` exits 0.
- [ ] Table shows all 9 columns from UI.md.
- [ ] "Load more" button fetches the next cursor page and appends rows.
- [ ] Loading state shows skeleton rows (5 × 9 cells).
- [ ] Empty state shows "No SQL events captured yet."
- [ ] Error state shows error icon + message.
- [ ] Status column uses `text-status-*` tokens + word (never color-only).
- [ ] Slow/error rows have a colored left border accent.
- [ ] Clicking a row opens the detail drawer.
- [ ] No `fetch` calls in the component (data via hooks only).
- [ ] No hardcoded status colors (`text-(red|green|amber|...)-[0-9]`).
- [ ] Dark mode renders correctly.

## Out of Scope

- Filters (Issue 071).
- Live WebSocket streaming / auto-prepend new events (future issue).
- Pause live updates (future issue).
- SQL Detail drawer content (future issue — drawer shows placeholder).
- Column sorting or resizing (future issue).
- Mobile card layout (future issue — horizontal scroll is the interim).

## Constraints

- Use `useSqlEvents()` from `@/lib/api/hooks` — no direct `fetch`.
- Use shadcn `Table`, `Badge`, `Skeleton`, `Button` from 065 baseline.
- No new dependencies.
