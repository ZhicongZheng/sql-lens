# Add HTTP server foundation

## Goal

Implement Issue 026 by adding a reusable HTTP server foundation for the SQL Lens Web/API surface.

This task establishes the server primitive that later REST, WebSocket, static web, auth, and dashboard tasks can compose. It should prove that SQL Lens can bind the configured web address, accept HTTP requests through an Axum router, attach request IDs, and shut down gracefully.

The implementation target is the `sql-lens-api` crate. The current CLI in `sql-lens-app` remains a startup validation command for now; it must not become a long-running runtime process in this task.

## Background

- `ISSUES.md` Issue 026 requires:
  - Server binds to `web.listen`.
  - Server shuts down gracefully.
  - Request IDs are attached to requests.
- `sql-lens-config::WebConfig` already exposes:
  - `listen: String`
  - `base_url: String`
  - `cors_origins: Vec<String>`
  - `static_dir: Option<String>`
  - `request_timeout_ms: u64`
- Backend specs currently state that `sql-lens-app` must not start proxy, API, storage, signal handling, hot reload, or async runtime services yet.
- Issue 027 separately owns `GET /api/v1/health`, so this task should not add a product health response.

## Requirements

- Add HTTP server foundation code in `crates/sql-lens-api`.
- Provide a small public server configuration that can be derived from `sql_lens_config::WebConfig`.
- Bind a `tokio::net::TcpListener` to the configured `web.listen` address.
- Expose the bound local address so tests and later runtime composition can discover it.
- Serve an Axum `Router` with graceful shutdown driven by a caller-provided shutdown future.
- Attach a request ID to each request.
- Propagate the request ID in a response header.
- Preserve an incoming request ID when the client sends one in the supported header.
- Generate a request ID when the client does not provide one.
- Use deterministic, dependency-light request ID generation suitable for request correlation, not security.
- Add lightweight tests covering bind, graceful shutdown, generated request ID, and incoming request ID propagation.
- Keep `sql-lens-api/src/lib.rs` thin if the crate gains more than one responsibility.

## Acceptance Criteria

- [x] `sql-lens-api` exposes a server primitive that binds to an address derived from `WebConfig.listen`.
- [x] A test proves binding works by using an ephemeral port and reading the bound local address.
- [x] A test proves the server exits successfully when the shutdown signal resolves.
- [x] A test proves a request without a request ID receives a generated request ID response header.
- [x] A test proves a request with a supported request ID header receives the same ID back.
- [x] The implementation does not add `GET /api/v1/health` or any SQL event endpoints.
- [x] The implementation does not make `sql-lens-app` start or block on an HTTP server.
- [x] `rtk cargo fmt --check`, `rtk cargo check --workspace`, `rtk cargo test --workspace`, and `rtk cargo clippy --workspace --all-targets -- -D warnings` pass.

## Out Of Scope

- Health endpoint response schema.
- API error response standardization.
- SQL event listing/detail APIs.
- WebSocket event streaming.
- Storage, proxy, capture, protocol, replay, plugin, auth, CORS, TLS, and static file integration.
- Production signal handling.
- CLI runtime composition or changing `sql-lens-app` from startup check to a long-running service.
- Cryptographically strong request IDs.
