import { QueryClient } from "@tanstack/react-query";

/**
 * Global QueryClient for SQL Lens.
 *
 * Defaults:
 *  - retry: 1          — local API, one retry is enough.
 *  - staleTime: 30s    — data is fresh for 30s, avoiding redundant refetches.
 *  - gcTime: 5min      — keep unused data in cache for 5 minutes.
 *
 * `useStatistics` overrides refetchInterval to 5s for live polling
 * (interim until WebSocket integration replaces it).
 */
export const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: 1,
      staleTime: 30_000,
      gcTime: 300_000,
    },
  },
});
