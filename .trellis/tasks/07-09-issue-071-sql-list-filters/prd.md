# Issue 071: Add SQL List filters

## Goal

Add filter controls to the SQL List page so users can narrow events by text,
protocol, status, database, user, and duration. Filter state is synced to
URL search params so filtered views are shareable and survive page refresh.

## Requirements

### R1 — Filter bar

A horizontal filter bar rendered above the SQL event table in `sql-events.tsx`,
using the shadcn `Input` and `Select` components from the 065 baseline.

| Filter | Control | API param | Notes |
|---|---|---|---|
| Text search | `Input` with `Search` icon | `q` | Free-text, searches SQL content |
| Protocol | `Select` | `protocol` | Options: `mysql` (hardcoded for now; protocols list is a follow-up) |
| Status | `Select` (multi-select not needed) | `status` | Options: `ok`, `slow`, `error`, `unknown` |
| Database | `Input` | `database` | Free-text, exact match |
| User | `Input` | `user` | Free-text, exact match |
| Duration min | `Input` (number) | `min_duration_ms` | Milliseconds |
| Duration max | `Input` (number) | `max_duration_ms` | Milliseconds |

On mobile (`< md`): the filter bar wraps to multiple lines naturally
(`flex-wrap`). No separate filter drawer for now (that's a future mobile
enhancement).

### R2 — URL state sync

Use React Router's `useSearchParams` to sync filter values to URL query
params. Param names match the API field names directly:

```
/sql?q=SELECT&protocol=mysql&status=error&database=app&user=admin&min_duration_ms=100
```

- When a filter changes, update the URL via `setSearchParams`.
- On page load, read initial filter values from the URL.
- Empty/default filter values are omitted from the URL (clean URLs).
- Changing filters resets the cursor and the accumulated `allItems` (starts
  a fresh query).

### R3 — Clear filters

A "Clear" button (shadcn `Button variant=ghost`) appears when any filter is
active. Clicking it resets all search params to empty, which clears the
filters and triggers a fresh unfiltered query.

### R4 — Filter → API wiring

The `useSqlEvents(params)` hook is called with the filter values derived from
URL search params. The hook's `queryKey` already includes `params`, so
changing filters automatically triggers a refetch with the correct API
query parameters.

### R5 — Active filter count

Show a small count badge next to the filter bar when filters are active
(e.g. "3 filters active"). This helps users notice that the view is filtered
when the filter bar is scrolled out of view on mobile.

## Acceptance Criteria

- [ ] `npm run build` exits 0.
- [ ] `npm run typecheck` exits 0.
- [ ] `npm run test` exits 0.
- [ ] Filter bar renders above the table with controls for all 7 filters.
- [ ] Changing a filter updates the URL search params and refetches data.
- [ ] URL with filter params loads the page with those filters pre-applied.
- [ ] Clear button resets all filters and URL params.
- [ ] Empty filter values are omitted from the URL.
- [ ] Active filter count is shown when any filter is active.
- [ ] Filters reset the accumulated event list (cursor starts fresh).
- [ ] No `fetch` calls in the component (data via hooks only).
- [ ] Dark mode renders correctly.

## Out of Scope

- Protocol dropdown populated from API (`useProtocols()` — follow-up).
- Filter drawer for mobile (future enhancement).
- Pause live updates (WebSocket issue).
- Duration range slider (future enhancement).

## Constraints

- Use `useSearchParams` from `react-router-dom` for URL state.
- Use shadcn `Input`, `Select`, `Button`, `Badge` from 065 baseline.
- No new dependencies.
