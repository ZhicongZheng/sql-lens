# Design — Issue 067: TanStack Query providers

## Installation

```
npm install @tanstack/react-query
```

No devtools (bundle size; can add later).

## QueryClient configuration

Created in a new `src/lib/query-client.ts`:

```ts
import { QueryClient } from "@tanstack/react-query";

export const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: 1,
      staleTime: 30_000,   // 30s
      gcTime: 300_000,     // 5min (default)
    },
  },
});
```

Separated into its own file so tests or future SSR can create fresh instances.

## Provider mount

In `src/main.tsx`, add `<QueryClientProvider client={queryClient}>`
wrapping the router. Updated nesting:

```
StrictMode > ThemeProvider > SidebarProvider > DetailDrawerProvider >
  QueryClientProvider > TooltipProvider > BrowserRouter > App + Toaster
```

Query provider doesn't depend on router, sidebar, or theme — it goes
outside TooltipProvider but inside the UI providers.

## Hook architecture

Hooks live in `src/lib/api/hooks/` (one file per domain). Each hook:

1. Imports the typed client function from `@/lib/api/client`.
2. Defines a stable `queryKey` array.
3. Returns the standard `UseQueryResult` shape.

Pattern:

```ts
import { useQuery } from "@tanstack/react-query";
import { getSqlEvents } from "@/lib/api/client";
import type { SqlEventQueryParams } from "@/types";

export function useSqlEvents(params?: SqlEventQueryParams) {
  return useQuery({
    queryKey: ["sql-events", params],
    queryFn: () => getSqlEvents(params),
  });
}
```

`queryKey` convention: `["resource-name", ...filterParams]`.

Special case: `useStatistics` adds `refetchInterval: 5_000` for polling
(until WebSocket replaces it).

Barrel at `src/lib/api/hooks/index.ts` re-exports all hooks.

## File changes

| File | Change |
|---|---|
| `package.json` | add `@tanstack/react-query` |
| `package-lock.json` | updated by npm |
| `src/lib/query-client.ts` | **new** — QueryClient singleton |
| `src/lib/api/hooks/use-health.ts` | **new** |
| `src/lib/api/hooks/use-sql-events.ts` | **new** |
| `src/lib/api/hooks/use-connections.ts` | **new** |
| `src/lib/api/hooks/use-statistics.ts` | **new** |
| `src/lib/api/hooks/use-protocols.ts` | **new** |
| `src/lib/api/hooks/use-replay.ts` | **new** |
| `src/lib/api/hooks/index.ts` | **new** — barrel |
| `src/main.tsx` | add QueryClientProvider |
| `README.md` | document Query config |

## Non-goals

- No devtools. No WebSocket. No optimistic updates. No infinite queries.
- No UI wiring (069 Dashboard and feature issues handle that).
