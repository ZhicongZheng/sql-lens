# Add HTTP Server Foundation Design

## Scope

This task adds HTTP server primitives to `crates/sql-lens-api`. It does not compose the full SQL Lens runtime and does not change CLI process behavior.

## Crate Boundary

`sql-lens-api` owns:

- HTTP server configuration for the API layer.
- Axum router construction for the empty API foundation.
- TCP listener binding for the web/API address.
- Graceful server shutdown.
- Request ID middleware.
- API-layer server errors.

`sql-lens-app` remains unchanged in this task. A later runtime task can decide how CLI startup, signal handling, config reload, proxy, API, and storage are composed.

## Dependencies

Add only the dependencies required for the server foundation:

- `axum` for the router and server integration.
- `tokio` for listener binding, async tests, and shutdown signaling.
- `tower` for middleware composition and test `ServiceExt` helpers if needed.
- `sql-lens-config` for deriving API server configuration from `WebConfig`.

Avoid adding `uuid`, `time`, `serde_json`, or storage/runtime composition dependencies in this task.

## Public API Shape

Proposed exports from `sql-lens-api`:

```rust
pub const REQUEST_ID_HEADER: &str = "x-request-id";

pub struct HttpServerConfig {
    pub listen: String,
    pub request_timeout_ms: u64,
}

impl From<&sql_lens_config::WebConfig> for HttpServerConfig;

pub struct BoundHttpServer {
    pub fn local_addr(&self) -> SocketAddr;
    pub fn router(&self) -> Router; // only if needed by tests or later composition
    pub async fn serve_with_shutdown(
        self,
        shutdown: impl Future<Output = ()> + Send + 'static,
    ) -> Result<(), HttpServerError>;
}

pub async fn bind_http_server(config: &HttpServerConfig) -> Result<BoundHttpServer, HttpServerError>;
pub fn router() -> Router;
```

The final names can be adjusted during implementation if Axum type constraints make a simpler shape clearer, but the public contract should stay small and explicit.

## Request ID Behavior

Use `x-request-id` as the request correlation header.

Middleware behavior:

1. If `x-request-id` is present and valid as a header value, preserve it.
2. If it is absent, generate an ID.
3. Store the selected ID in request extensions if useful for later handlers.
4. Add the selected ID to the response headers.

Generated IDs do not need to be globally cryptographically strong. A process-local atomic counter plus a fixed prefix is sufficient for this foundation task and avoids unnecessary dependencies. A later tracing/security task can replace this implementation if stronger correlation IDs are required.

## Router Behavior

The foundation router may be empty except for middleware. It should still be valid to serve and to test by adding a test-only route or by layering the middleware over a small router in unit tests.

Do not add `/api/v1/health`; Issue 027 owns that endpoint and its schema.

## Error Handling

Add an API-local `HttpServerError` with variants for:

- bind failures with the configured address and IO source.
- local address lookup failures if needed.
- serve failures with IO source.

Implement `Display` and `std::error::Error` so `sql-lens-app` or future runtime composition can surface errors cleanly.

Address parse errors can be handled by `TcpListener::bind(&str)` as IO errors, so this task does not need a separate address parser unless the implementation requires one.

## Compatibility

- No existing public API is removed.
- No config keys change.
- No CLI behavior changes.
- Existing app tests that expect startup validation to exit must continue to pass.

## Validation Strategy

Tests should stay local to `sql-lens-api` and avoid binding fixed ports. Use `127.0.0.1:0` to request an ephemeral port.

Required test coverage:

- `HttpServerConfig` derives from `WebConfig`.
- Binding returns a local address with an allocated port.
- Server exits after shutdown is triggered.
- Request ID is generated when absent.
- Incoming request ID is propagated when present.

Workspace validation remains the final gate.
