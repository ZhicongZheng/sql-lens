# Standardize REST error responses design

## Boundary

Implement in `crates/sql-lens-api`.

This task standardizes API error construction, response serialization, request ID injection, and router fallback behavior. It must not implement auth, RBAC, rate limiting, proxy readiness checks, storage health checks, panic recovery, frontend code, or protocol parsing.

## Current State

Current code already has the right rough pieces:

- `request_id::attach_request_id` generates or preserves `x-request-id`, stores `RequestId` in request extensions, runs the router, and writes the response header.
- `ApiEndpointError` maps some handler errors into JSON envelopes.
- Existing endpoints return `Result<Json<T>, ApiEndpointError>`.

The gap:

- `ApiEndpointError::into_response()` cannot see request extensions, so it writes `request_id: None`.
- Unknown routes currently use Axum's default 404 response instead of SQL Lens' documented JSON envelope.
- Only `bad_request` and `not_found` helper constructors exist today.
- The documented error-code-to-status mapping is not tested as one contract.

## Recommended Approach

Use a response-extension marker for API errors and let the existing request ID middleware centralize request ID injection.

### Why This Approach

There are three viable designs:

1. Add `Extension<RequestId>` to every handler and manually attach request IDs to errors.
2. Buffer and parse every error response body in middleware, then rewrite JSON.
3. Add a typed response extension to `ApiEndpointError` responses, then have middleware rebuild only marked API error bodies with the request ID.

Recommendation: option 3.

It keeps handler signatures clean, avoids JSON body parsing in middleware, keeps the current `Result<Json<T>, ApiEndpointError>` pattern, and centralizes request ID behavior in the middleware that already owns request ID generation.

Axum 0.8 supports middleware that receives `Request` and `Next`, runs the next service, and modifies the `Response`. Axum responses also support extensions. This gives us a small typed handoff between `ApiEndpointError` and `attach_request_id`.

## Error Response Model

Keep the public JSON shape:

```json
{
  "error": {
    "code": "BAD_REQUEST",
    "message": "Invalid duration filter",
    "request_id": "sql-lens-0000000000000001",
    "details": {
      "field": "min_duration_ms"
    }
  }
}
```

Internal API structs:

```rust
pub(crate) struct ApiErrorEnvelope {
    error: ApiErrorBody,
}

pub(crate) struct ApiErrorBody {
    code: String,
    message: String,
    request_id: Option<String>,
    details: BTreeMap<String, String>,
}

#[derive(Clone)]
pub(crate) struct ApiErrorResponseParts {
    body: ApiErrorBody,
}
```

`ApiErrorResponseParts` is not a public API type. It is a typed marker placed in response extensions so request ID middleware can rebuild the body without parsing JSON.

## `ApiEndpointError` Contract

`ApiEndpointError` should own:

- HTTP status.
- `ApiErrorCode`.
- Message.
- Details.

It should expose constructors only for mappings used by current handlers, and keep all documented mappings centralized in helper functions. Add more constructors when runtime code first needs them.

```rust
impl ApiEndpointError {
    pub(crate) fn bad_request(message: impl Into<String>, field: impl Into<String>) -> Self;
    pub(crate) fn not_found(message: impl Into<String>, key: impl Into<String>, value: impl Into<String>) -> Self;
}
```

Internally, use mapping functions that cover every documented `ApiErrorCode`:

```rust
fn api_error_status(code: ApiErrorCode) -> StatusCode;
fn api_error_code_name(code: ApiErrorCode) -> &'static str;
```

This keeps the code-name and status mapping testable without needing real auth/storage/proxy runtime conditions or unused future-facing constructors.

## Request ID Injection Flow

Data flow:

```text
Request
  -> attach_request_id middleware
    -> choose generated or valid incoming request ID
    -> insert RequestId extension
    -> route handler
      -> returns ApiEndpointError
      -> IntoResponse creates JSON body with request_id = None
      -> IntoResponse inserts ApiErrorResponseParts extension
    -> attach_request_id sees ApiErrorResponseParts
    -> rebuilds the JSON body with request_id = Some(...)
    -> writes x-request-id header
  -> Response
```

Implementation helper:

```rust
pub(crate) fn with_request_id(response: Response, request_id: &RequestId) -> Response
```

This helper lives in `api_error.rs` so the error body shape remains encapsulated there. `request_id.rs` calls it after `next.run(request).await`.

If no `ApiErrorResponseParts` extension exists, the helper returns the response unchanged.

## Request ID Validity

Request ID body values are strings.

To guarantee that JSON always receives a usable string:

- Preserve incoming `x-request-id` only if `HeaderValue::to_str()` succeeds.
- Generate a new request ID when the incoming header is missing or not valid visible ASCII.
- Use the same normalized request ID for the response header, request extension, and error JSON body.

`RequestId` should expose:

```rust
impl RequestId {
    pub fn as_header_value(&self) -> &HeaderValue;
    pub(crate) fn as_str(&self) -> &str;
}
```

This may require storing both the `HeaderValue` and a `String`, or deriving the `String` once when constructing `RequestId`.

## Router Fallback

Add an API fallback handler:

```rust
async fn api_not_found(uri: Uri) -> ApiEndpointError {
    ApiEndpointError::not_found("Route not found", "path", uri.path().to_owned())
}
```

Register it on `router_with_state`:

```rust
Router::new()
    .merge(...)
    .fallback(api_not_found)
    .layer(Extension(state))
    .layer(middleware::from_fn(attach_request_id))
```

The fallback should cover unmatched routes and produce the same envelope/request ID behavior as handler errors.

Method-not-allowed responses are not in the documented `ApiErrorCode` list. Do not add a new public `METHOD_NOT_ALLOWED` code in this task. If future product requirements need standardized 405 responses, add a dedicated `ApiErrorCode` variant in a separate task.

## Compatibility

Existing handlers can keep returning `ApiEndpointError`.

Existing successful response DTOs should not change.

Existing tests that only asserted the `x-request-id` header should continue passing; new tests should assert JSON body request IDs for error responses.

## Tests

Add or update tests for:

- Mapping every documented `ApiErrorCode` to HTTP status and string code.
- Generated request ID appears in both header and JSON body for a representative handler error.
- Incoming valid `x-request-id` appears in both header and JSON body.
- Invalid incoming `x-request-id` is replaced by a generated one.
- Unknown route returns `NOT_FOUND` JSON envelope with request ID.
- Existing representative endpoint error, such as invalid `limit`, still returns `BAD_REQUEST` with details.

## Rollback

If response-extension rewriting causes unexpected Axum behavior, fall back to the more explicit but noisier approach: require handlers that can fail to extract `Extension<RequestId>` and call `error.with_request_id(request_id)`.

Do not use body parsing middleware as the first implementation because it is more fragile and adds unnecessary runtime work for a typed in-process contract.
