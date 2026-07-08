// API base URL configuration. This module is config-only: it reads the
// VITE_API_BASE_URL environment variable and exposes the resolved base.
//
// No endpoint-coupled client functions live here — typed API client functions
// are deferred to Issue 066 ("Add frontend API client"). The skeleton must not
// hardcode any concrete backend coupling (fetch/XHR/WebSocket).
//
// Default matches the API listener recommended default in ARCHITECTURE.md
// (127.0.0.1:5173). In dev, Vite serves the frontend on 5174 and the Rust
// backend serves the API on 5173; set VITE_API_BASE_URL to override.
const resolvedBase =
  import.meta.env.VITE_API_BASE_URL ?? "http://127.0.0.1:5173";

export const apiBaseUrl: string = resolvedBase.replace(/\/$/, "");
