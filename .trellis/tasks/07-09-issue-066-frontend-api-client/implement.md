# Implement — Issue 066: Frontend API client

## 1. Vitest setup

- [ ] `cd crates/sql-lens-app/web && npm install -D vitest @testing-library/jest-dom jsdom`.
- [ ] Create `vitest.config.ts`:
      ```ts
      import { defineConfig } from "vitest/config";
      import path from "node:path";
      export default defineConfig({
        resolve: { alias: { "@": path.resolve(__dirname, "./src") } },
        test: { environment: "jsdom" },
      });
      ```
- [ ] Add to `package.json` scripts: `"test": "vitest run"`.
- [ ] Verify: `npm run test` runs (no tests yet, exits 0).

## 2. TypeScript types

- [ ] Replace `src/types/index.ts` placeholder with real interfaces from
      API.md:
      - `ApiErrorCode` (string literal union of 9 error codes).
      - `ApiError` { code, message, request_id?, details? }.
      - `PaginatedResponse<T>` { items: T[], next_cursor?: string }.
      - `SqlEvent` (all fields from API.md /sql-events response).
      - `SqlEventSummary` (streaming subset).
      - `SqlEventQueryParams` (all query params, all optional).
      - `SqlConnection` (all fields from API.md /connections response).
      - `Statistics` (window, qps, error_rate, slow_count, latency_ms,
        active_connections).
      - `Protocol` (name, status, databases).
      - `HealthResponse` (status, version, uptime_ms).
      - `ReplayPreviewRequest` ({ event_id } | { sql }).
      - `ReplayPreviewResponse` (source, event_id?, sql, is_mutation,
        warning?).
- [ ] `npm run typecheck` → exit 0.

## 3. Error handling

- [ ] Create `src/lib/api/errors.ts`:
      - `ApiClientError extends Error` with `code`, `status`, `requestId?`,
        `details?`.
      - Constructor takes `ApiError` + HTTP status.
      - `isApiClientError(err): err is ApiClientError` type guard.
- [ ] No changes to existing `config.ts` — it stays as-is.

## 4. Typed REST client

- [ ] Create `src/lib/api/client.ts`:
      - Import `apiBaseUrl` from `@/lib/api/config`.
      - Import types from `@/types`.
      - Internal helper: `async function request<T>(path, init?): Promise<T>`
        that builds URL, calls fetch, handles errors.
      - Export functions:
        - `getHealth()`
        - `getSqlEvents(params?: SqlEventQueryParams)`
        - `getSqlEvent(id: string)`
        - `getConnections()`
        - `getConnection(id: string)`
        - `getStatistics(window?: string)`
        - `getProtocols()`
        - `previewReplay(req: ReplayPreviewRequest)`
- [ ] Only `client.ts` contains `fetch(` calls (grep assertion).

## 5. Tests

- [ ] Create `src/lib/api/__tests__/client.test.ts`:
      - Test 1 (success): mock `fetch` returning 200 with valid `SqlEvent`
        response, call `getSqlEvent("test-id")`, verify returned object
        matches expected shape.
      - Test 2 (error): mock `fetch` returning 404 with `ApiError` body
        `{error:{code:"NOT_FOUND",message:"...",request_id:"..."}}`, call
        `getSqlEvent("missing")`, verify `ApiClientError` is thrown with
        `code === "NOT_FOUND"` and `status === 404`.
- [ ] `npm run test` → exit 0 with ≥2 tests passing.

## 6. Barrel exports

- [ ] Create `src/lib/api/index.ts` that re-exports everything from
      `client.ts` and `errors.ts` (so consumers import from `@/lib/api`).
- [ ] Verify no other file imports `fetch` directly.

## Validation gates

- [ ] `npm run build` → exit 0.
- [ ] `npm run typecheck` → exit 0.
- [ ] `npm run test` → exit 0 (≥2 tests).
- [ ] `grep -rn "fetch(" src/` → only in `client.ts` and `client.test.ts`.
- [ ] `grep -rn "/api/v1" src/` → only in `client.ts`.
- [ ] No Monaco/ECharts/TanStack Query/next-themes in `package.json`.

## Rollback

`git checkout -- crates/sql-lens-app/web/` reverts all changes. No Rust
touched.
