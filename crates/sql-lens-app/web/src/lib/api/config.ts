// API base URL configuration. This module is config-only: it reads the
// VITE_API_BASE_URL environment variable and exposes the resolved base.
//
// No endpoint-coupled client functions live here — typed API client functions
// are deferred to Issue 066 ("Add frontend API client"). The skeleton must not
// hardcode any concrete backend coupling (fetch/XHR/WebSocket).
//
export function resolveApiBaseUrl(
  configuredBase: string | undefined,
  browserOrigin: string | undefined,
): string {
  return (configuredBase ?? browserOrigin ?? "http://127.0.0.1:5173").replace(
    /\/$/,
    "",
  );
}

export const apiBaseUrl = resolveApiBaseUrl(
  import.meta.env.VITE_API_BASE_URL,
  typeof window === "undefined" ? undefined : window.location.origin,
);
