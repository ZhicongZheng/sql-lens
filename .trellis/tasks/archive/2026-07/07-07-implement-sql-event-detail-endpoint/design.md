# Implement SQL Event Detail Endpoint Design

## Scope

Add `GET /api/v1/sql-events/{id}` inside the existing `sql_events` module.

## Endpoint

```http
GET /api/v1/sql-events/{id}
```

Path parameter:

```rust
id: String
```

The handler wraps it as `sql_lens_core::SqlEventId` and calls `RingBufferStore::get`.

## Response DTO

Add:

```rust
pub struct SqlEventDetailResponse {
    // all summary fields
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
    pub normalized_sql: Option<String>,
    pub expanded_sql: Option<String>,
    pub fingerprint: Option<String>,
    pub parameters: Vec<SqlParameterResponse>,
    pub timings: QueryTimingResponse,
    pub rows: Option<RowsSummaryResponse>,
    pub error: Option<ErrorSummaryResponse>,
    pub metadata: ProtocolMetadataResponse,
}
```

Use dedicated DTOs for:

- `SqlParameterResponse`
- `SqlParameterValueResponse`
- `QueryTimingResponse`
- `ErrorSummaryResponse`

The list endpoint can keep its existing summary DTO. Mapping helpers for status, kind, rows, and metadata should be reused.

## Parameter Value Mapping

`SqlParameterValueResponse` should be an untagged or externally tagged DTO that keeps enough type information for UI rendering. Recommended shape:

```json
{
  "type": "integer",
  "value": 42
}
```

Binary and unsupported values should remain strings.

## Error Mapping

Missing event:

- HTTP 404
- `ApiErrorCode::NotFound`
- message: `SQL event not found`
- details: `{ "id": "<requested-id>" }`

Request ID remains in the response header. Error body can continue to use `request_id: null` until request ID body plumbing is designed.

## Compatibility

- `GET /api/v1/sql-events` behavior must not change.
- `router()` and `router_with_state(ApiState)` remain stable.
- No storage API changes should be needed.
