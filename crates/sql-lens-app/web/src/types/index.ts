// Type definitions for the SQL Lens REST API.
// Source of truth: project root API.md. Hand-written (no codegen).

// ---------------------------------------------------------------------------
// Error model
// ---------------------------------------------------------------------------

/** Error codes defined in API.md § Error Codes. */
export type ApiErrorCode =
  | "BAD_REQUEST"
  | "UNAUTHORIZED"
  | "FORBIDDEN"
  | "NOT_FOUND"
  | "CONFLICT"
  | "RATE_LIMITED"
  | "INTERNAL"
  | "STORAGE_UNAVAILABLE"
  | "PROXY_NOT_READY";

/** Envelope returned by the API on non-2xx responses. */
export interface ApiError {
  code: ApiErrorCode;
  message: string;
  request_id?: string;
  details?: Record<string, unknown>;
}

/** The full error response body. */
export interface ApiErrorResponse {
  error: ApiError;
}

// ---------------------------------------------------------------------------
// Generic helpers
// ---------------------------------------------------------------------------

/** Paginated list response used by /sql-events, /connections, /protocols. */
export interface PaginatedResponse<T> {
  items: T[];
  next_cursor?: string;
}

// ---------------------------------------------------------------------------
// SQL Events
// ---------------------------------------------------------------------------

/** Row counts attached to a SQL event. */
export interface SqlEventRows {
  affected: number;
  returned: number;
}

/** Protocol-specific metadata (e.g. MySQL command + statement_id). */
export type SqlEventMetadata = Record<string, Record<string, unknown>>;

// ---------------------------------------------------------------------------
// SQL Parameters
// ---------------------------------------------------------------------------

/**
 * Parameter value — a tagged object matching the backend `SqlParameterValue`
 * enum. The `type` field identifies the variant; `value` carries the payload.
 *
 * Variants: null, integer, unsigned, float, boolean, string, date, time,
 * timestamp, json, binary_summary, unsupported.
 */
export interface SqlParameterValue {
  type: string;
  value: string | number | boolean | null;
}

/** A single SQL statement parameter. */
export interface SqlParameter {
  index: number;
  name?: string;
  value: SqlParameterValue;
  redacted: boolean;
}

/** Full SQL event from GET /api/v1/sql-events/{id}. */
export interface SqlEvent {
  id: string;
  timestamp: string;
  target_name: string;
  protocol: string;
  database_type: string;
  connection_id: string;
  client_addr: string;
  backend_addr: string;
  user: string;
  database: string;
  kind: string;
  status: string;
  duration_ms: number;
  original_sql: string;
  expanded_sql: string;
  fingerprint: string;
  rows: SqlEventRows;
  parameters: SqlParameter[];
  metadata: SqlEventMetadata;
}

/** Summary subset used in the WebSocket/sql-events stream. */
export interface SqlEventSummary {
  id: string;
  timestamp: string;
  target_name: string;
  protocol: string;
  status: string;
  duration_ms: number;
  sql_preview: string;
}

/** Query parameters for GET /api/v1/sql-events. All optional. */
export interface SqlEventQueryParams {
  limit?: number;
  cursor?: string;
  target_name?: string;
  protocol?: string;
  database_type?: string;
  database?: string;
  user?: string;
  client_addr?: string;
  status?: string;
  min_duration_ms?: number;
  max_duration_ms?: number;
  q?: string;
  fingerprint?: string;
  from?: string;
  to?: string;
}

// ---------------------------------------------------------------------------
// Connections
// ---------------------------------------------------------------------------

/** Connection from GET /api/v1/connections/{id}. */
export interface SqlConnection {
  id: string;
  target_name: string;
  protocol: string;
  database_type: string;
  client_addr: string;
  backend_addr: string;
  user: string;
  database: string;
  state: string;
  connected_at: string;
  last_activity_at: string;
  bytes_in: number;
  bytes_out: number;
  query_count: number;
}

// ---------------------------------------------------------------------------
// Statistics
// ---------------------------------------------------------------------------

/** Latency percentiles. */
export interface LatencyMs {
  p50: number;
  p95: number;
  p99: number;
}

/** Statistics from GET /api/v1/statistics. */
export interface Statistics {
  window: string;
  qps: number;
  error_rate: number;
  slow_count: number;
  latency_ms: LatencyMs;
  active_connections: number;
}

// ---------------------------------------------------------------------------
// Protocols
// ---------------------------------------------------------------------------

/** Protocol from GET /api/v1/protocols. */
export interface Protocol {
  name: string;
  status: "supported" | "planned";
  databases: string[];
}

// ---------------------------------------------------------------------------
// Health
// ---------------------------------------------------------------------------

/** Health response from GET /api/v1/health. */
export interface HealthResponse {
  status: string;
  version: string;
  uptime_ms: number;
}

// ---------------------------------------------------------------------------
// Replay
// ---------------------------------------------------------------------------

/** Request body for POST /api/v1/replay/preview. Exactly one of event_id or sql. */
export type ReplayPreviewRequest =
  | { event_id: string; sql?: never }
  | { event_id?: never; sql: string };

/** Response from POST /api/v1/replay/preview. */
export interface ReplayPreviewResponse {
  source: string;
  event_id?: string;
  sql: string;
  is_mutation: boolean;
  warning?: string;
}
