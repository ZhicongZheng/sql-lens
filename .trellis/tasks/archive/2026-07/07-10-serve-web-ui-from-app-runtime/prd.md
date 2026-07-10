# Serve built web UI from app runtime

## Goal

Let one `sql-lens` process serve both the developer UI and its API/WebSocket
endpoints from a configured, prebuilt Vite distribution directory.

## Confirmed Facts

- `WebConfig` already exposes `static_dir`, but `HttpServerConfig` drops it and
  the Axum router currently falls back to an API JSON 404 response.
- The web application builds to `dist/`; the Vite development server uses port
  5174 while the Rust HTTP server defaults to port 5173.
- The frontend currently defaults API and WebSocket requests to a fixed 5173
  origin, which would break a static UI served from a different configured port.

## Requirements

- When `web.static_dir` is configured and contains a built `index.html`, the
  Rust HTTP server serves static frontend assets from that directory.
- Client-side routes such as `/dashboard` fall back to `index.html`.
- Existing `/api/*`, `/ws/*`, and health routes retain their current behavior
  and are never replaced by the SPA fallback.
- A configured directory that is missing or lacks `index.html` fails startup
  with a clear server configuration error.
- Built frontend clients use the current browser origin by default; Vite dev
  mode proxies API and WebSocket paths to the Rust server. `VITE_API_BASE_URL`
  remains an explicit override.
- Documentation shows the build-and-run flow using one `sql-lens` process.

## Acceptance Criteria

- [x] `HttpServerConfig` carries `WebConfig.static_dir` into router creation.
- [x] A configured static directory serves `/`, hashed assets, and SPA routes.
- [x] Existing API and WebSocket routes remain reachable; unknown API paths
  return API JSON 404 responses.
- [x] Invalid configured static directories produce a typed startup error.
- [x] Frontend tests cover same-origin URL construction and Vite development
  proxy settings cover `/api` and `/ws`.
- [x] Rust and frontend tests cover the affected behavior, and the runtime
  documentation describes the one-process flow.

## Out of Scope

- Embedding frontend assets into the executable.
- Building the frontend automatically from Cargo or at application startup.
- Release artifact, installer, Docker, or Homebrew packaging automation.
