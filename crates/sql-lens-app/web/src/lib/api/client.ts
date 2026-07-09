import { apiBaseUrl } from "@/lib/api/config";
import { ApiClientError } from "@/lib/api/errors";
import type {
  ApiErrorResponse,
  HealthResponse,
  PaginatedResponse,
  Protocol,
  ReplayPreviewRequest,
  ReplayPreviewResponse,
  SqlConnection,
  SqlEvent,
  SqlEventQueryParams,
  Statistics,
} from "@/types";

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/** Build a URL with optional query parameters. */
function buildUrl(
  path: string,
  params?: Record<string, string | number | undefined>,
): string {
  const url = new URL(`/api/v1${path}`, apiBaseUrl);
  if (params) {
    for (const [key, value] of Object.entries(params)) {
      if (value !== undefined) {
        url.searchParams.set(key, String(value));
      }
    }
  }
  return url.toString();
}

/**
 * Core fetch wrapper. Only place in the codebase that calls `fetch`.
 * Throws ApiClientError on non-2xx responses.
 */
async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const url = buildUrl(path);
  let response: Response;

  try {
    response = await fetch(url, {
      headers: { "Content-Type": "application/json" },
      ...init,
    });
  } catch (err) {
    // Network error — wrap as INTERNAL
    throw new ApiClientError(
      {
        code: "INTERNAL",
        message: err instanceof Error ? err.message : "Network error",
      },
      0,
    );
  }

  if (!response.ok) {
    let apiError: ApiErrorResponse["error"];
    try {
      const body = (await response.json()) as ApiErrorResponse;
      apiError = body.error;
    } catch {
      apiError = {
        code: "INTERNAL",
        message: `HTTP ${response.status}`,
      };
    }
    throw new ApiClientError(apiError, response.status);
  }

  return (await response.json()) as T;
}

/** Build a URL with query params and fetch. */
async function requestWithParams<T>(
  path: string,
  params?: Record<string, string | number | undefined>,
): Promise<T> {
  const url = buildUrl(path, params);
  let response: Response;

  try {
    response = await fetch(url, {
      headers: { "Content-Type": "application/json" },
    });
  } catch (err) {
    throw new ApiClientError(
      {
        code: "INTERNAL",
        message: err instanceof Error ? err.message : "Network error",
      },
      0,
    );
  }

  if (!response.ok) {
    let apiError: ApiErrorResponse["error"];
    try {
      const body = (await response.json()) as ApiErrorResponse;
      apiError = body.error;
    } catch {
      apiError = {
        code: "INTERNAL",
        message: `HTTP ${response.status}`,
      };
    }
    throw new ApiClientError(apiError, response.status);
  }

  return (await response.json()) as T;
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/** GET /api/v1/health */
export async function getHealth(): Promise<HealthResponse> {
  return request<HealthResponse>("/health");
}

/** GET /api/v1/sql-events */
export async function getSqlEvents(
  params?: SqlEventQueryParams,
): Promise<PaginatedResponse<SqlEvent>> {
  return requestWithParams<PaginatedResponse<SqlEvent>>(
    "/sql-events",
    params as Record<string, string | number | undefined> | undefined,
  );
}

/** GET /api/v1/sql-events/:id */
export async function getSqlEvent(id: string): Promise<SqlEvent> {
  return request<SqlEvent>(`/sql-events/${encodeURIComponent(id)}`);
}

/** GET /api/v1/connections */
export async function getConnections(): Promise<
  PaginatedResponse<SqlConnection>
> {
  return request<PaginatedResponse<SqlConnection>>("/connections");
}

/** GET /api/v1/connections/:id */
export async function getConnection(id: string): Promise<SqlConnection> {
  return request<SqlConnection>(`/connections/${encodeURIComponent(id)}`);
}

/** GET /api/v1/statistics */
export async function getStatistics(window?: string): Promise<Statistics> {
  return requestWithParams<Statistics>(
    "/statistics",
    window ? { window } : undefined,
  );
}

/** GET /api/v1/protocols */
export async function getProtocols(): Promise<PaginatedResponse<Protocol>> {
  return request<PaginatedResponse<Protocol>>("/protocols");
}

/** POST /api/v1/replay/preview */
export async function previewReplay(
  req: ReplayPreviewRequest,
): Promise<ReplayPreviewResponse> {
  return request<ReplayPreviewResponse>("/replay/preview", {
    method: "POST",
    body: JSON.stringify(req),
  });
}
