import type { ApiError, ApiErrorCode } from "@/types";

/**
 * Thrown by API client functions on non-2xx responses or network errors.
 * Wraps the structured error body from the API.
 */
export class ApiClientError extends Error {
  /** The API error code (e.g. NOT_FOUND, INTERNAL). */
  readonly code: ApiErrorCode;

  /** The HTTP status code of the response. */
  readonly status: number;

  /** Server-provided request ID for tracing, if present. */
  readonly requestId?: string;

  /** Additional error details from the API response, if present. */
  readonly details?: Record<string, unknown>;

  constructor(apiError: ApiError, status: number) {
    super(apiError.message);
    this.name = "ApiClientError";
    this.code = apiError.code;
    this.status = status;
    this.requestId = apiError.request_id;
    this.details = apiError.details;
  }
}

/** Type guard for ApiClientError. */
export function isApiClientError(err: unknown): err is ApiClientError {
  return err instanceof ApiClientError;
}
