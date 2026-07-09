# Design — Issue 066: Frontend API client

## Location

`crates/sql-lens-app/web/src/lib/api/` (client + errors) and
`crates/sql-lens-app/web/src/types/` (interfaces).

## Type strategy

Types are hand-written from `API.md` (project root). Source of truth is the
JSON examples and field descriptions in that file. No codegen — the OpenAPI
yaml doesn't exist yet and won't for v1.

Types go in `src/types/index.ts` (replacing the Issue 064 placeholder barrel).
Single file for now; split into `types/sql-event.ts`, `types/connection.ts`
etc. only if the file exceeds ~200 lines.

## Client architecture

`src/lib/api/client.ts` exports pure async functions, one per endpoint.
Each function:

1. Reads `apiBaseUrl` from `@/lib/api/config`.
2. Builds the full URL: `${apiBaseUrl}/api/v1/<path>`.
3. Calls `fetch` with appropriate method/headers.
4. Checks `response.ok`; if not, parses error body as `ApiError` and throws
   `ApiClientError`.
5. Parses JSON body and returns the typed result.

No class instantiation, no singleton — pure functions. This makes testing
trivial (mock `fetch` per call).

`src/lib/api/errors.ts` exports:

```ts
class ApiClientError extends Error {
  code: ApiErrorCode
  status: number
  requestId?: string
  details?: Record<string, unknown>
}

type ApiErrorCode = "BAD_REQUEST" | "UNAUTHORIZED" | "FORBIDDEN" | ...

function isApiClientError(err: unknown): err is ApiClientError
```

## Query parameter handling

`getSqlEvents` accepts `SqlEventQueryParams` (all optional). The function
builds a `URLSearchParams` from the non-undefined fields and appends it.
No special serialization — all values are strings or numbers (coerced to
string).

## Vitest setup

- `npm install -D vitest @testing-library/jest-dom jsdom`.
- `vitest.config.ts` extending `vite.config.ts` with `test.environment:
  "jsdom"`.
- `package.json` script: `"test": "vitest run"`.
- Tests in `src/lib/api/__tests__/client.test.ts`.
- Mock pattern: `vi.stubGlobal("fetch", mockFn)` per test.

## File changes

| File | Change |
|---|---|
| `src/types/index.ts` | replace placeholder with real type interfaces |
| `src/lib/api/errors.ts` | **new** — ApiClientError class + type guard |
| `src/lib/api/client.ts` | **new** — typed fetch functions |
| `src/lib/api/__tests__/client.test.ts` | **new** — Vitest tests |
| `vitest.config.ts` | **new** — Vitest config |
| `package.json` | add vitest + jsdom dev deps, `test` script |

## Non-goals

- No TanStack Query (067). No WebSocket client. No UI data wiring.
- No OpenAPI yaml. No codegen. No MSW.
- No `replay/execute` endpoint (needs mutation confirmation UX first).
