# Implement SQL Event List Endpoint Design

## Scope

This task adds `GET /api/v1/sql-events` and completes the missing ring-buffer filters required by `API.md`.

The implementation touches:

- `sql-lens-storage`: add `client_addr` and `fingerprint` to `SqlEventFilter`.
- `sql-lens-api`: add API state, query parsing, DTO mapping, endpoint route, and endpoint tests.

It does not wire `sql-lens-app` into a long-running runtime.

## Storage Changes

Extend `SqlEventFilter`:

```rust
pub struct SqlEventFilter {
    pub protocol: Option<ProtocolName>,
    pub database_type: Option<DatabaseType>,
    pub database: Option<String>,
    pub user: Option<String>,
    pub client_addr: Option<String>,
    pub status: Option<CaptureStatus>,
    pub min_duration: Option<DurationMillis>,
    pub max_duration: Option<DurationMillis>,
    pub text: Option<String>,
    pub fingerprint: Option<String>,
    pub from: Option<Timestamp>,
    pub to: Option<Timestamp>,
}
```

Matching semantics:

- `client_addr` matches `SqlEvent.client_addr` exactly.
- `fingerprint` matches `SqlEvent.fingerprint.as_deref()` exactly.
- All existing filters keep their current semantics.

No new storage dependencies are required.

## API State

Add a small API state type:

```rust
pub struct ApiState {
    event_store: std::sync::Arc<tokio::sync::RwLock<RingBufferStore>>,
}
```

Public constructors:

```rust
impl ApiState {
    pub fn new(event_store: RingBufferStore) -> Self;
    pub fn event_store(&self) -> Arc<RwLock<RingBufferStore>>;
}

impl Default for ApiState;
```

Default state uses an empty ring buffer with a pragmatic default capacity so existing `router()` callers keep working. Future runtime composition can pass a configured store through `router_with_state`.

Router shape:

```rust
pub fn router() -> Router;
pub fn router_with_state(state: ApiState) -> Router;
```

`router()` remains a convenience wrapper around `router_with_state(ApiState::default())`.

## Query Contract

Endpoint:

```http
GET /api/v1/sql-events
```

Supported query fields:

```rust
pub struct SqlEventListQueryParams {
    pub limit: Option<usize>,
    pub cursor: Option<String>,
    pub protocol: Option<String>,
    pub database_type: Option<String>,
    pub database: Option<String>,
    pub user: Option<String>,
    pub client_addr: Option<String>,
    pub status: Option<String>,
    pub min_duration_ms: Option<u64>,
    pub max_duration_ms: Option<u64>,
    pub q: Option<String>,
    pub fingerprint: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
}
```

Limits:

- default `limit`: 100
- maximum `limit`: 500
- `limit = 0`: HTTP 400
- `limit > 500`: clamp to 500 for now, so clients cannot accidentally over-fetch but valid requests remain useful

Cursor:

- Encode `RingBufferTimelineCursor { before_sequence }` as `seq_<before_sequence>`.
- Decode only `seq_<u64>`.
- Invalid cursor: HTTP 400 with `BAD_REQUEST`.

Status:

- Supported values: `ok`, `slow`, `error`, `unknown`.
- Invalid status: HTTP 400 with `BAD_REQUEST`.

Duration and timestamp validation:

- Storage still validates `min_duration_ms <= max_duration_ms`.
- Storage still validates `from <= to` using existing sortable timestamp string semantics.
- Validation errors map to HTTP 400.

## Response Contract

Top-level response:

```rust
pub struct SqlEventListResponse {
    pub items: Vec<SqlEventSummaryResponse>,
    pub next_cursor: Option<String>,
}
```

Item DTO:

```rust
pub struct SqlEventSummaryResponse {
    pub id: String,
    pub timestamp: String,
    pub protocol: String,
    pub database_type: String,
    pub connection_id: String,
    pub client_addr: String,
    pub backend_addr: String,
    pub user: Option<String>,
    pub database: Option<String>,
    pub kind: String,
    pub status: String,
    pub duration_ms: u64,
    pub original_sql: String,
    pub expanded_sql: Option<String>,
    pub fingerprint: Option<String>,
    pub rows: Option<RowsSummaryResponse>,
    pub metadata: ProtocolMetadataResponse,
}
```

`SqlEventKind` maps to snake_case strings:

- `Query` -> `query`
- `StatementPrepare` -> `statement_prepare`
- `StatementExecute` -> `statement_execute`
- `StatementClose` -> `statement_close`
- `ConnectionCommand` -> `connection_command`
- `Unknown` -> `unknown`

`CaptureStatus` maps to lowercase strings:

- `Ok` -> `ok`
- `Slow` -> `slow`
- `Error` -> `error`
- `Unknown` -> `unknown`

Metadata response should match `API.md`'s protocol-keyed object:

```json
{
  "mysql": {
    "command": "COM_STMT_EXECUTE",
    "statement_id": 12
  }
}
```

Use `BTreeMap<String, BTreeMap<String, MetadataValueResponse>>` to keep output deterministic in tests. `MetadataValueResponse` should serialize as an untagged string/integer/unsigned/float/boolean value.

## Error Contract

Add API-local error response helpers for this endpoint:

```json
{
  "error": {
    "code": "BAD_REQUEST",
    "message": "Invalid duration filter",
    "request_id": null,
    "details": {
      "field": "min_duration_ms"
    }
  }
}
```

The current request ID middleware stores an API request ID in extensions, but the response can initially return `request_id: null` if plumbing it into error bodies would complicate the endpoint. The response header still contains `x-request-id`.

Use core `ApiErrorCode` names for API error codes and do not invent unrelated names.

## Trade-Offs

- The API returns DTOs instead of serializing `SqlEvent` directly because core enums currently serialize as Rust variant names, while `API.md` expects lowercase/snake_case strings.
- `seq_<u64>` cursor encoding is intentionally simple and local to the ring buffer. Later storage backends can replace cursor internals behind the same query string contract.
- This task uses concrete `RingBufferStore` state instead of a repository trait. A trait can be introduced when a second storage backend is actually wired into API reads.
- Query parsing stays endpoint-local. Shared API query utilities can be extracted after a second endpoint repeats the same patterns.

## Compatibility

- Existing `router()` remains available.
- Existing health endpoint and request ID behavior remain unchanged.
- Existing storage timeline behavior remains unchanged when new filters are absent.
