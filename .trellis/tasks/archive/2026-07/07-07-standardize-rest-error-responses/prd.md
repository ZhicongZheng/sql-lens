# Standardize REST error responses

## Goal

Implement Issue 033 design: standardize SQL Lens REST API error responses so every API error uses the documented `ApiError` envelope and includes the request ID in both the response header and JSON body.

## Background

- `API.md` documents error responses as:

```json
{
  "error": {
    "code": "BAD_REQUEST",
    "message": "Invalid duration filter",
    "request_id": "req_01J00000000000000000000000",
    "details": {
      "field": "min_duration_ms"
    }
  }
}
```

- `ApiEndpointError` already serializes API errors with `code`, `message`, `request_id`, and `details`.
- Current `ApiEndpointError::into_response()` sets `request_id` to `None` because `IntoResponse` does not have request extensions.
- Current request ID middleware attaches `x-request-id` headers and makes a `RequestId` extension available to handlers.
- Axum fallback responses for unmatched routes are currently framework-generated and do not use the SQL Lens error envelope.

## Requirements

- Ensure documented API error codes have HTTP status mappings:
  - `BAD_REQUEST` -> 400
  - `UNAUTHORIZED` -> 401
  - `FORBIDDEN` -> 403
  - `NOT_FOUND` -> 404
  - `CONFLICT` -> 409
  - `RATE_LIMITED` -> 429
  - `INTERNAL` -> 500
  - `STORAGE_UNAVAILABLE` -> 503
  - `PROXY_NOT_READY` -> 503
- Include request ID in REST error response bodies when the request ID middleware is installed.
- Preserve `x-request-id` response header behavior.
- Standardize unmatched API route 404 responses.
- Keep endpoint handlers simple; they should continue returning `Result<Json<T>, ApiEndpointError>` where possible.
- Keep the error shape protocol-neutral.
- Do not add frontend work.
- Do not implement auth, rate limiting, proxy readiness, or storage availability checks; only provide standardized mappings and currently needed constructors.

## Acceptance Criteria

- [x] `ApiEndpointError` supports constructors or mapping paths for all documented API error codes.
- [x] Unit tests verify every documented error code maps to the expected HTTP status and string code.
- [x] Error responses include `request_id` in the JSON body when routed through `router()` / `router_with_state()`.
- [x] Error responses still include the `x-request-id` response header.
- [x] Incoming valid `x-request-id` values are preserved in both header and body.
- [x] Generated request IDs are included in both header and body.
- [x] Unmatched routes return the documented `NOT_FOUND` envelope.
- [x] Representative handler errors, such as invalid query parameters, return the documented envelope.
- [x] Existing REST endpoint success behavior remains unchanged.
- [x] `cargo fmt --check` passes.
- [x] `cargo test --workspace` passes.
- [x] `cargo clippy --workspace --all-targets -- -D warnings` passes.

## Out of Scope

- Panic catching.
- Tracing/logging correlation changes.
- Full authentication and authorization implementation.
- Rate limiting implementation.
- Storage/proxy runtime health integration.
- OpenAPI generation.
- Changing the public `ApiErrorCode` enum.
