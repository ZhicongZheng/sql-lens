# Issue 066: Add frontend API client

## Goal

Build a typed REST API client for the SQL Lens backend, establishing the
frontend data layer. This unblocks Issue 067 (TanStack Query providers),
Issue 069 (Dashboard with real data), SQL List, Connections, and all
downstream feature pages that consume backend data.

## Requirements

### R1 — TypeScript types from API.md

Hand-write TypeScript interfaces for all REST response/request shapes defined
in the project root `API.md`. Place under `src/types/`.

Core types (matching API.md schema names):

- `SqlEvent` — full event with id, timestamp, target_name, protocol,
  database_type, connection_id, client_addr, backend_addr, user, database,
  kind, status, duration_ms, original_sql, expanded_sql, fingerprint, rows
  ({affected, returned}), metadata (protocol-specific JSON).
- `SqlEventSummary` — the WebSocket/streaming subset (id, timestamp,
  target_name, protocol, status, duration_ms, sql_preview).
- `SqlConnection` — id, target_name, protocol, database_type, client_addr,
  backend_addr, user, database, state, connected_at, last_activity_at,
  bytes_in, bytes_out, query_count.
- `Statistics` — window, qps, error_rate, slow_count, latency_ms ({p50, p95,
  p99}), active_connections.
- `Protocol` — name, status (`"supported"` | `"planned"`), databases
  (string[]).
- `ApiError` — code (enum of error codes from API.md), message, request_id?,
  details? (Record<string, unknown>).
- `ReplayPreviewRequest` — exactly one of `event_id` or `sql`.
- `ReplayPreviewResponse` — source, event_id?, sql, is_mutation, warning?.
- `PaginatedResponse<T>` — generic wrapper for `{ items: T[], next_cursor?:
  string }`.

Error code enum: `BAD_REQUEST`, `UNAUTHORIZED`, `FORBIDDEN`, `NOT_FOUND`,
`CONFLICT`, `RATE_LIMITED`, `INTERNAL`, `STORAGE_UNAVAILABLE`,
`PROXY_NOT_READY`.

### R2 — Typed REST client

Create `src/lib/api/client.ts` exporting a typed client. This module is the
ONLY place that calls `fetch`. It reads `apiBaseUrl` from `@/lib/api/config`.

Endpoints (one function per resource):

- `getHealth(): Promise<HealthResponse>`
- `getSqlEvents(params?: SqlEventQueryParams): Promise<PaginatedResponse<SqlEvent>>`
- `getSqlEvent(id: string): Promise<SqlEvent>`
- `getConnections(): Promise<PaginatedResponse<SqlConnection>>`
- `getConnection(id: string): Promise<SqlConnection>`
- `getStatistics(window?: string): Promise<Statistics>`
- `getProtocols(): Promise<PaginatedResponse<Protocol>>`
- `previewReplay(req: ReplayPreviewRequest): Promise<ReplayPreviewResponse>`

All functions throw a typed `ApiClientError` on non-2xx responses, which
wraps the `ApiError` from the response body.

### R3 — Error handling

- `ApiClientError` class in `src/lib/api/errors.ts`: extends `Error`, carries
  `code` (ApiErrorCode), `status` (HTTP status), `requestId?`, `details?`.
- Client functions catch `fetch` errors (network) and non-2xx responses, wrap
  them in `ApiClientError`, and re-throw.
- `isApiClientError(err): err is ApiClientError` type guard exported.

### R4 — Vitest test infrastructure

- Install `vitest` + `@testing-library/jest-dom` as dev dependencies.
- Add `npm run test` script to `package.json`.
- Create `vitest.config.ts` (or use `vite.config.ts` test config).
- Write tests in `src/lib/api/__tests__/client.test.ts`:
  - One successful request test (mock `fetch`, verify typed response).
  - One failed request test (mock `fetch` returning 404 with ApiError body,
    verify `ApiClientError` is thrown with correct code/status).
- The tests mock `globalThis.fetch` — no MSW, no real backend.

## Acceptance Criteria

- [ ] `npm run build` exits 0.
- [ ] `npm run typecheck` exits 0.
- [ ] `npm run test` exits 0 with ≥2 tests (1 success, 1 error).
- [ ] All types under `src/types/` match API.md response shapes.
- [ ] `src/lib/api/client.ts` is the only file that calls `fetch` (grep
      assertion: `grep -rn "fetch(" src/` shows only client.ts and
      client.test.ts).
- [ ] `ApiClientError` carries the error code from the API response body.
- [ ] No `next-themes`, Monaco, ECharts, or TanStack Query added.
- [ ] No `/api/v1` literal outside `client.ts`.

## Out of Scope

- TanStack Query providers (Issue 067).
- WebSocket client (future issue).
- Real data wiring in UI components (follow-up feature issues).
- OpenAPI yaml generation (future issue).
- Replay execute endpoint (only preview is included; execute needs
  mutation confirmation UX first).

## Constraints

- Package manager: npm.
- Types are hand-written from API.md (no codegen tooling).
- `fetch` calls are confined to `src/lib/api/client.ts` and its test file.
- Keep the dependency surface lean — Vitest + jest-dom only; no MSW.
