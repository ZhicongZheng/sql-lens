# Design: Serve Built Web UI From App Runtime

## Boundaries

`sql-lens-config` continues to own the optional `web.static_dir` value.
`sql-lens-api` owns validation and serving of the directory because it already
owns the Axum router. `sql-lens-app` only passes the converted HTTP config.
The Vite application remains an independently built asset set.

## Request Routing

When no static directory is configured, the router preserves its current API
JSON fallback. When one is configured, startup validates both the directory and
its `index.html`, then configures a `tower_http::services::ServeDir` fallback.

Known API and WebSocket routes remain registered before the fallback. Explicit
catch-all routes for `/api/*` and `/ws/*` preserve JSON API errors instead of
returning the frontend shell for a mistyped service endpoint. All other
unmatched paths fall back to `index.html` so React Router can resolve client
routes.

## Frontend Origin Contract

Without `VITE_API_BASE_URL`, browser clients use `window.location.origin`.
Vite development mode proxies `/api` and `/ws` to the local Rust listener, so
the same relative-origin behavior works with hot reload. An explicit
`VITE_API_BASE_URL` remains available for non-default development topologies.

## Failure Behavior

Static serving is opt-in. A configured but missing directory or `index.html`
returns a typed `HttpServerError` before binding succeeds. No static directory
keeps API-only behavior for tests and programmatic users.

## Compatibility

No REST, WebSocket, or configuration schema changes are required. The new
`HttpServerConfig.static_dir` field is an internal Rust application contract;
all existing constructors must explicitly use `None`.

## Deferred Work

The first release can distribute `sql-lens` alongside `web/dist`. Embedding
assets and orchestrating the Node build belong to release packaging, after the
runtime serving contract is stable.
