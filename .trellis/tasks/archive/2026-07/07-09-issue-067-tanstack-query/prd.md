# Issue 067: Add TanStack Query providers

## Goal

Configure TanStack Query as the frontend's server-state layer, providing
automatic caching, background refetching, retry, and stale-while-revalidate
for all API calls. This unblocks Issue 069 (Dashboard with live statistics)
and all subsequent feature pages that consume backend data.

## Requirements

### R1 — TanStack Query installation and provider

- Install `@tanstack/react-query` as a dependency.
- Create a `QueryClient` with documented default options:
  - `retry`: 1 (one retry on failure; the API is local-first, so retries are
    cheap but excessive retries add noise).
  - `staleTime`: 30 seconds (data is considered fresh for 30s; background
    refetches happen after that). This balances freshness with avoiding
    redundant requests for a developer tool that doesn't need sub-second
    freshness.
  - `gcTime`: 5 minutes (default; keep unused data in cache for 5 minutes
    so switching between tabs doesn't refetch immediately).
- Mount `<QueryClientProvider>` at the app root in `src/main.tsx`, inside
  the existing providers but **outside** `<BrowserRouter>` (Query provider
  doesn't depend on router context).

Provider nesting order:
`StrictMode > ThemeProvider > SidebarProvider > DetailDrawerProvider >
QueryClientProvider > TooltipProvider > BrowserRouter > App + Toaster`

### R2 — Query hooks for API resources

Create typed `useQuery`/`useMutation` hooks in `src/lib/api/hooks/` (one file
per resource domain). Each hook:

- Uses the typed client functions from `@/lib/api/client.ts`.
- Defines a stable `queryKey` array (e.g. `["sql-events", params]`).
- Returns the standard `UseQueryResult` / `UseMutationResult` shape.

Initial hooks (covering the existing API client functions):

- `src/lib/api/hooks/use-health.ts`:
  - `useHealth()` — `queryKey: ["health"]`, calls `getHealth()`.
- `src/lib/api/hooks/use-sql-events.ts`:
  - `useSqlEvents(params?)` — `queryKey: ["sql-events", params]`, calls
    `getSqlEvents(params)`.
  - `useSqlEvent(id)` — `queryKey: ["sql-events", id]`, calls
    `getSqlEvent(id)`.
- `src/lib/api/hooks/use-connections.ts`:
  - `useConnections()` — `queryKey: ["connections"]`, calls
    `getConnections()`.
  - `useConnection(id)` — `queryKey: ["connections", id]`, calls
    `getConnection(id)`.
- `src/lib/api/hooks/use-statistics.ts`:
  - `useStatistics(window?)` — `queryKey: ["statistics", window]`, calls
    `getStatistics(window)`.
  - `refetchInterval`: 5 seconds (statistics should poll for live updates
    until WebSocket integration replaces it).
- `src/lib/api/hooks/use-protocols.ts`:
  - `useProtocols()` — `queryKey: ["protocols"]`, calls `getProtocols()`.
- `src/lib/api/hooks/use-replay.ts`:
  - `usePreviewReplay()` — `useMutation` calling `previewReplay(req)`.

Barrel export at `src/lib/api/hooks/index.ts`.

### R3 — Documentation

- Add a "Server State" section to `README.md` documenting:
  - The QueryClient defaults (retry, staleTime, gcTime).
  - How to add a new query hook (pattern).
  - Where hooks live (`src/lib/api/hooks/`).

## Acceptance Criteria

- [ ] `npm run build` exits 0.
- [ ] `npm run typecheck` exits 0.
- [ ] `npm run test` exits 0 (existing tests still pass).
- [ ] `QueryClientProvider` mounted at app root with documented defaults.
- [ ] At least `useSqlEvents` and `useStatistics` hooks exist and use typed
      client functions from `@/lib/api/client`.
- [ ] `useStatistics` has `refetchInterval: 5000`.
- [ ] No `fetch` calls outside `src/lib/api/client.ts` (grep assertion).
- [ ] No Monaco, ECharts, or `next-themes` added.
- [ ] README documents the Query configuration.

## Out of Scope

- Real data wiring in UI components (that's 069 Dashboard and subsequent
  feature issues).
- WebSocket integration for live events (future issue; polling via
  `refetchInterval` is the interim solution).
- Optimistic updates, infinite queries, or query invalidation patterns
  (follow-up when features need them).

## Constraints

- Package manager: npm.
- TanStack Query v5 (current stable).
- Do not add TanStack Query Devtools (they add bundle size; can be a
  separate follow-up if desired).
