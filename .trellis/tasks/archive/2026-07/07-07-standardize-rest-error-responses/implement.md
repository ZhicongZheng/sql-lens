# Standardize REST error responses plan

## Checklist

- [x] Read backend specs before implementation.
- [x] Update `ApiEndpointError` current constructors and status/code mapping helpers.
- [x] Add internal API error response marker stored in response extensions.
- [x] Add an `api_error` helper that rebuilds marked error responses with a request ID.
- [x] Normalize request ID construction so JSON body values are valid strings.
- [x] Update `attach_request_id` to call the API error rewrite helper before inserting the response header.
- [x] Add router fallback for unmatched routes using `ApiEndpointError::not_found`.
- [x] Add mapping tests for all documented error codes.
- [x] Add request ID body/header tests for generated and incoming request IDs.
- [x] Add unmatched route `NOT_FOUND` envelope test.
- [x] Update representative existing endpoint error tests to assert body request ID where useful.
- [x] Update backend spec with standardized REST error response contract.
- [x] Run `rtk cargo fmt --check`.
- [x] Run `rtk cargo test --workspace`.
- [x] Run `rtk cargo clippy --workspace --all-targets -- -D warnings`.

## Validation Commands

```bash
rtk cargo fmt --check
rtk cargo test -p sql-lens-api
rtk cargo test --workspace
rtk cargo clippy --workspace --all-targets -- -D warnings
```

## Risk Notes

- Request ID injection must be centralized; avoid adding request ID plumbing to every handler.
- Do not parse response JSON in middleware unless the typed response-extension design fails.
- Preserve success response behavior.
- Preserve the existing `x-request-id` header behavior.
- Do not add a new public error code for 405 in this task.

## Review Gate

Before implementation starts, confirm this design direction:

- `ApiEndpointError` marks its own responses via response extensions.
- `attach_request_id` rewrites only those marked API error responses.
- Router fallback standardizes unmatched route 404 responses.
