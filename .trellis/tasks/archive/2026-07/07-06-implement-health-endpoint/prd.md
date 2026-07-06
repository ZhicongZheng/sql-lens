# Implement health endpoint

## Goal

Implement Issue 027: add GET /api/v1/health returning status, version, and uptime without requiring storage.

The endpoint gives local tools, tests, and future UI/runtime composition a minimal API readiness signal. It should be part of the `sql-lens-api` router introduced by Issue 026.

## Background

- `API.md` defines `GET /api/v1/health`.
- The documented response shape is:

```json
{
  "status": "ok",
  "version": "0.1.0",
  "uptime_ms": 120000
}
```

- Issue 026 already added the Axum router foundation and request ID middleware.
- The current `sql-lens-app` CLI must still remain startup-check-only in this task.

## Requirements

- Add `GET /api/v1/health` to the `sql-lens-api` router.
- Return HTTP 200 for the health endpoint.
- Return JSON fields `status`, `version`, and `uptime_ms`.
- Set `status` to `"ok"` for the first implementation.
- Set `version` from the API crate package version.
- Compute `uptime_ms` from in-process server/router state using `std::time::Instant`.
- Ensure the endpoint works without storage, proxy, capture, protocol, database, auth, or plugin state.
- Preserve Issue 026 request ID behavior on the health response.
- Add a test that covers response status and JSON schema.
- Do not change `sql-lens-app` runtime behavior.

## Acceptance Criteria

- [x] `GET /api/v1/health` returns HTTP 200.
- [x] Response JSON contains `status: "ok"`.
- [x] Response JSON contains the crate version.
- [x] Response JSON contains a numeric `uptime_ms`.
- [x] Endpoint works without storage data.
- [x] Health responses still include the request ID header.
- [x] Test covers the response schema.
- [x] Existing request ID and HTTP foundation tests still pass.
- [x] `sql-lens-app` remains startup-check-only.

## Out Of Scope

- Deep readiness checks for storage, proxy, capture, protocol adapters, plugins, or database connectivity.
- Kubernetes-style liveness/readiness split.
- Authentication and authorization.
- OpenAPI generation.
- Runtime startup or signal handling in `sql-lens-app`.
