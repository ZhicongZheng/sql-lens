# Implement — Issue 067: TanStack Query providers

## 1. Install TanStack Query

- [ ] `cd crates/sql-lens-app/web && npm install @tanstack/react-query`.
- [ ] Verify: `grep "@tanstack/react-query" package.json` shows the dep.

## 2. QueryClient

- [ ] Create `src/lib/query-client.ts`:
      ```ts
      import { QueryClient } from "@tanstack/react-query";
      export const queryClient = new QueryClient({
        defaultOptions: {
          queries: {
            retry: 1,
            staleTime: 30_000,
            gcTime: 300_000,
          },
        },
      });
      ```

## 3. Provider mount

- [ ] Edit `src/main.tsx`:
      - Import `QueryClientProvider` from `@tanstack/react-query`.
      - Import `queryClient` from `@/lib/query-client`.
      - Wrap: `...DetailDrawerProvider > QueryClientProvider >
        TooltipProvider >...`

## 4. Query hooks

- [ ] Create `src/lib/api/hooks/use-health.ts`:
      `useHealth()` — `queryKey: ["health"]`, `queryFn: getHealth`.
- [ ] Create `src/lib/api/hooks/use-sql-events.ts`:
      `useSqlEvents(params?)` — `queryKey: ["sql-events", params]`,
      `queryFn: () => getSqlEvents(params)`.
      `useSqlEvent(id)` — `queryKey: ["sql-events", id]`,
      `queryFn: () => getSqlEvent(id)`.
- [ ] Create `src/lib/api/hooks/use-connections.ts`:
      `useConnections()` — `queryKey: ["connections"]`,
      `queryFn: getConnections`.
      `useConnection(id)` — `queryKey: ["connections", id]`,
      `queryFn: () => getConnection(id)`.
- [ ] Create `src/lib/api/hooks/use-statistics.ts`:
      `useStatistics(window?)` — `queryKey: ["statistics", window]`,
      `queryFn: () => getStatistics(window)`,
      `refetchInterval: 5_000`.
- [ ] Create `src/lib/api/hooks/use-protocols.ts`:
      `useProtocols()` — `queryKey: ["protocols"]`, `queryFn: getProtocols`.
- [ ] Create `src/lib/api/hooks/use-replay.ts`:
      `usePreviewReplay()` — `useMutation({ mutationFn: previewReplay })`.
- [ ] Create `src/lib/api/hooks/index.ts` — barrel re-exporting all hooks.

## 5. Documentation

- [ ] Add "Server State" section to `README.md`:
      - QueryClient defaults (retry: 1, staleTime: 30s, gcTime: 5min).
      - `useStatistics` polls every 5s (interim until WebSocket).
      - How to add a new hook (pattern: import client fn, define queryKey,
        return useQuery).
      - Hook location: `src/lib/api/hooks/`.

## Validation gates

- [ ] `npm run build` → exit 0.
- [ ] `npm run typecheck` → exit 0.
- [ ] `npm run test` → exit 0 (existing tests still pass).
- [ ] `grep -rn "fetch(" src/` → only in `client.ts` and `client.test.ts`.
- [ ] `grep -rn "next-themes\|monaco\|echarts" package.json` → no matches.
- [ ] `QueryClientProvider` present in `main.tsx` with `queryClient` prop.

## Rollback

`git checkout -- crates/sql-lens-app/web/` reverts. No Rust touched.
