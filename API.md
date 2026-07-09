# SQL Lens API

## Overview

SQL Lens exposes REST APIs for queryable state and WebSocket APIs for live updates.

The API is local-first by default. Authentication can be disabled for local-only development and enabled for shared environments.

Base path:

```text
/api
```

API versioning:

```text
/api/v1
```

## REST Resources

### Health

```http
GET /api/v1/health
```

Response:

```json
{
  "status": "ok",
  "version": "0.1.0",
  "uptime_ms": 120000
}
```

### SQL Events

```http
GET /api/v1/sql-events
```

Query parameters:

- `limit`
- `cursor`
- `target_name`
- `protocol`
- `database_type`
- `database`
- `user`
- `client_addr`
- `status`
- `min_duration_ms`
- `max_duration_ms`
- `q`
- `fingerprint`
- `from`
- `to`

Response:

```json
{
  "items": [
    {
      "id": "evt_01J00000000000000000000000",
      "timestamp": "2026-07-03T12:00:00Z",
      "target_name": "mysql-local",
      "protocol": "mysql",
      "database_type": "mysql",
      "connection_id": "conn_01J00000000000000000000000",
      "client_addr": "127.0.0.1:51000",
      "backend_addr": "127.0.0.1:3306",
      "user": "app",
      "database": "app",
      "kind": "statement_execute",
      "status": "ok",
      "duration_ms": 3.4,
      "original_sql": "SELECT * FROM users WHERE id = ?",
      "expanded_sql": "SELECT * FROM users WHERE id = 42",
      "fingerprint": "select * from users where id = ?",
      "rows": {
        "affected": 0,
        "returned": 1
      },
      "metadata": {
        "mysql": {
          "command": "COM_STMT_EXECUTE",
          "statement_id": 12
        }
      }
    }
  ],
  "next_cursor": "cursor_abc"
}
```

### SQL Event Detail

```http
GET /api/v1/sql-events/{id}
```

Returns the full event, parameter list, timings, result summary, error summary, and metadata.

### Connections

```http
GET /api/v1/connections
GET /api/v1/connections/{id}
```

Connection response:

```json
{
  "id": "conn_01J00000000000000000000000",
  "target_name": "mysql-local",
  "protocol": "mysql",
  "database_type": "mysql",
  "client_addr": "127.0.0.1:51000",
  "backend_addr": "127.0.0.1:3306",
  "user": "app",
  "database": "app",
  "state": "ready",
  "connected_at": "2026-07-03T12:00:00Z",
  "last_activity_at": "2026-07-03T12:01:00Z",
  "bytes_in": 4096,
  "bytes_out": 8192,
  "query_count": 32
}
```

### Statistics

```http
GET /api/v1/statistics
```

Parameters:

- `window`: optional. Current live endpoint supports `1m` and `60s`; omitted defaults to `1m`.

Planned future filters:

- `protocol`
- `database_type`
- `database`
- `user`

Response:

```json
{
  "window": "1m",
  "qps": 120.5,
  "error_rate": 0.01,
  "slow_count": 4,
  "latency_ms": {
    "p50": 2.1,
    "p95": 8.4,
    "p99": 22.0
  },
  "active_connections": 18
}
```

### Protocols

```http
GET /api/v1/protocols
```

Response:

```json
{
  "items": [
    {
      "name": "mysql",
      "status": "supported",
      "databases": ["mysql", "starrocks", "tidb", "doris"]
    },
    {
      "name": "postgresql",
      "status": "planned",
      "databases": ["postgresql"]
    }
  ]
}
```

### Replay

```http
POST /api/v1/replay/preview
POST /api/v1/replay/execute
```

Replay execution must require explicit confirmation, especially for mutating SQL.

## WebSocket API

### SQL Events Stream

```text
GET /ws/sql
```

Client subscription:

```json
{
  "type": "subscribe",
  "version": 1,
  "filters": {
    "target_name": "mysql-local",
    "protocol": "mysql",
    "status": ["ok", "error", "slow"],
    "database": "app",
    "min_duration_ms": 10,
    "max_duration_ms": 500
  }
}
```

Current implementation requires a valid `subscribe` message before sending live events. `filters` is optional; when omitted, the subscriber receives all future SQL events. Supported filters are:

- `protocol`: exact protocol name.
- `target_name`: exact configured proxy target name.
- `status`: one or more of `ok`, `slow`, `error`, or `unknown`.
- `database`: exact database name.
- `min_duration_ms`: inclusive minimum duration in milliseconds.
- `max_duration_ms`: inclusive maximum duration in milliseconds.

All provided filter fields use AND semantics. Multiple `status` values use OR semantics. Invalid filter fields or values return a subscription error and keep the socket open while the server waits for a later valid `subscribe` message.

Server event:

```json
{
  "type": "sql_event.created",
  "version": 1,
  "payload": {
    "id": "evt_01J00000000000000000000000",
    "timestamp": "2026-07-03T12:00:00Z",
    "target_name": "mysql-local",
    "protocol": "mysql",
    "status": "ok",
    "duration_ms": 3.4,
    "sql_preview": "SELECT * FROM users WHERE id = 42"
  }
}
```

Subscription error:

```json
{
  "type": "subscription.error",
  "version": 1,
  "payload": {
    "code": "INVALID_FILTER",
    "message": "invalid subscription filter",
    "field": "filters.status"
  }
}
```

### Statistics Stream

```text
GET /ws/statistics
```

Server event:

```json
{
  "type": "statistics.updated",
  "version": 1,
  "payload": {
    "qps": 120.5,
    "active_connections": 18,
    "error_rate": 0.01
  }
}
```

## JSON Schema Strategy

Schemas should live near API code and be generated into OpenAPI.

Core schema names:

- `SqlEvent`
- `SqlEventSummary`
- `SqlParameter`
- `Connection`
- `Statistics`
- `ApiError`
- `ReplayPreview`
- `ReplayRequest`

`metadata` fields are protocol-specific JSON objects with documented sub-schemas where possible.

## Error Codes

| Code | HTTP | Meaning |
| --- | --- | --- |
| `BAD_REQUEST` | 400 | Invalid query or body |
| `UNAUTHORIZED` | 401 | Authentication required |
| `FORBIDDEN` | 403 | Authenticated but not allowed |
| `NOT_FOUND` | 404 | Resource not found |
| `CONFLICT` | 409 | State conflict |
| `RATE_LIMITED` | 429 | Request rate limit exceeded |
| `INTERNAL` | 500 | Unexpected server error |
| `STORAGE_UNAVAILABLE` | 503 | Storage backend unavailable |
| `PROXY_NOT_READY` | 503 | Proxy service not ready |

Error response:

```json
{
  "error": {
    "code": "BAD_REQUEST",
    "message": "Invalid duration filter",
    "request_id": "req_01J00000000000000000000000",
    "details": {
      "field": "min_duration_ms"
    }
  }
}
```

## OpenAPI

The project should publish an OpenAPI document for every release:

```text
docs/openapi/sql-lens.v1.yaml
```

Rules:

- REST endpoints must be represented in OpenAPI.
- WebSocket messages should be documented with JSON schemas.
- Breaking changes require a version bump.
- Generated client types should be possible but not required for v1.
