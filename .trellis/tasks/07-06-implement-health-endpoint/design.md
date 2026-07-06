# Implement Health Endpoint Design

## Scope

Add a minimal `GET /api/v1/health` endpoint inside `sql-lens-api`.

## Crate Boundary

`sql-lens-api` owns the response schema and route registration.

The endpoint must not depend on:

- storage
- proxy
- capture
- protocol adapters
- plugins
- frontend assets
- app runtime composition

## Data Model

Add a health response type:

```rust
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_ms: u64,
}
```

The type should derive `Serialize` and `Deserialize` so tests and later schema tooling can reuse it.

## State

Use a small health state:

```rust
pub struct HealthState {
    started_at: std::time::Instant,
    version: &'static str,
}
```

Default state uses:

```rust
env!("CARGO_PKG_VERSION")
```

`uptime_ms` is computed from `started_at.elapsed().as_millis()` and saturated to `u64::MAX` if needed.

## Routing

`router()` should register:

```text
GET /api/v1/health
```

Request ID middleware from Issue 026 should remain applied to the full router, including health.

## Compatibility

- Existing `/missing` request ID tests should continue to receive 404 plus `x-request-id`.
- No CLI runtime behavior changes.
- No storage or proxy state is required to serve health.

## Validation

Add an Axum/Tower unit test that calls `/api/v1/health` and asserts:

- status is HTTP 200.
- content decodes into `HealthResponse`.
- `status == "ok"`.
- `version == env!("CARGO_PKG_VERSION")`.
- `uptime_ms` is numeric.
- `x-request-id` exists.
