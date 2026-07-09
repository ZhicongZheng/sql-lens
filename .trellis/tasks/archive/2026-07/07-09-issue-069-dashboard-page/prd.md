# Issue 069: Build Dashboard page

## Goal

Replace the Issue 065 demo stub in `dashboard.tsx` with a real dashboard that
displays live statistics from the backend API via `useStatistics()`. This is
the first page to consume real data through the TanStack Query layer.

## Requirements

### R1 — Statistics cards

Display the following metrics in a responsive card grid (3 columns on desktop,
1 on mobile):

- **QPS** — queries per second (from `statistics.qps`). Format: one decimal
  place.
- **Latency** — p50 / p95 / p99 (from `statistics.latency_ms`). Display as
  three sub-values in one card, formatted in milliseconds with one decimal.
- **Active Connections** (from `statistics.active_connections`).
- **Slow SQL Count** (from `statistics.slow_count`). Use `text-status-slow`
  token.
- **Error Rate** (from `statistics.error_rate`). Display as percentage with
  two decimal places. Use `text-status-error` token.

Each card uses the shadcn `Card` component. Metric value is large/bold, label
is small/muted. Status-related cards (slow, error) use the `text-status-*`
tokens for the value — never color-only, pair with the label word.

### R2 — Loading state

While `useStatistics()` is loading (`isLoading`), show `Skeleton` placeholders
inside each card — a rectangular skeleton matching the metric value size and
a smaller skeleton for the label. The card grid structure stays visible so the
layout doesn't shift when data arrives.

### R3 — Error state

If `useStatistics()` returns an error (`isError`), show an alert-style message
inside the card area: an icon + "Failed to load statistics" + the error
message from `error.message`. Use `text-status-error` for the icon. The rest
of the page (topbar, sidebar) remains functional.

### R4 — Empty state

If `useStatistics()` returns successfully but with zero/null values (e.g. no
traffic yet), display the metric cards with "—" as the value and "No data yet"
as a muted subtitle in the QPS card. This covers the cold-start scenario.

### R5 — Remove 065 demo content

Remove all Issue 065 demo surface code from `dashboard.tsx`:
- Sample `Table` with hardcoded SQL events.
- `Tabs` block.
- `Tooltip` + `Dialog` + `toast()` demo triggers.
- `PageStub` import and usage.
- The `StatusBadge` helper component.

The dashboard becomes a pure statistics page. The `PageStub` component stays
in the codebase (other route stubs use it) — only remove its usage from
dashboard.

### R6 — Responsive layout

- Desktop (`≥ md`): 3-column grid of cards (`grid-cols-3`).
- Tablet: 2-column (`grid-cols-2`).
- Mobile (`< sm`): 1-column (`grid-cols-1`).
- Cards have consistent height within each row.

## Acceptance Criteria

- [ ] `npm run build` exits 0.
- [ ] `npm run typecheck` exits 0.
- [ ] `npm run test` exits 0.
- [ ] Dashboard displays QPS, latency (p50/p95/p99), active connections, slow
      count, and error rate from `useStatistics()`.
- [ ] Loading state shows `Skeleton` placeholders in the card grid.
- [ ] Error state shows a visible error message.
- [ ] Empty/null values show "—" with a "No data yet" note.
- [ ] Status cards use `text-status-slow` / `text-status-error` tokens (not
      hardcoded colors).
- [ ] Layout is responsive: 3→2→1 columns.
- [ ] 065 demo content (sample table, tabs, dialog, toast triggers) is
      removed.
- [ ] No `fetch` calls in `dashboard.tsx` (data comes from hooks).
- [ ] Dark mode renders correctly.

## Out of Scope

- Protocol mix, top fingerprints, error timeline (require additional API
  endpoints not yet implemented).
- Click-to-filter interactions (require SQL List filter wiring).
- Time window selector (requires backend window parameter support beyond
  `1m`/`60s`).
- ECharts charts (own issue).

## Constraints

- Use `useStatistics()` from `@/lib/api/hooks` — no direct `fetch`.
- Use shadcn `Card` and `Skeleton` from 065 baseline.
- No new dependencies.
