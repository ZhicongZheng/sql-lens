# Implement Connections Endpoint Design

## Scope

This task adds:

- `sql-lens-storage::ConnectionStore`
- `GET /api/v1/connections`
- `GET /api/v1/connections/{id}`

It does not wire live proxy runtime updates into the store. Runtime composition can pass the same store to proxy/API in a later task.

## Storage Boundary

Add `crates/sql-lens-storage/src/connection_store.rs`.

Public types:

```rust
pub struct ConnectionStore;

impl ConnectionStore {
    pub fn new(capacity: std::num::NonZeroUsize) -> Self;
    pub fn upsert(&mut self, connection: sql_lens_core::ConnectionInfo) -> ConnectionUpsertOutcome;
    pub fn list_recent(&self, limit: std::num::NonZeroUsize) -> Vec<sql_lens_core::ConnectionInfo>;
    pub fn get(&self, id: &sql_lens_core::ConnectionId) -> Option<&sql_lens_core::ConnectionInfo>;
    pub fn len(&self) -> usize;
    pub fn capacity(&self) -> usize;
    pub fn is_empty(&self) -> bool;
}

pub struct ConnectionUpsertOutcome {
    pub stored_connection_id: sql_lens_core::ConnectionId,
    pub replaced_existing: bool,
    pub evicted_connection_id: Option<sql_lens_core::ConnectionId>,
}
```

Storage semantics:

- Capacity is `NonZeroUsize`.
- `upsert` stores the latest `ConnectionInfo` for a `ConnectionId`.
- If the ID already exists, replace it and move it to the newest position.
- If the ID is new and the store is full, evict the oldest-updated connection.
- `list_recent` returns cloned connections newest-first.
- `get` returns a borrowed retained connection.
- No async runtime dependency in storage.

## API State

Extend `ApiState`:

```rust
pub const DEFAULT_CONNECTION_STORE_CAPACITY: usize = 10_000;

impl ApiState {
    pub fn new(event_store: RingBufferStore) -> Self;
    pub fn with_stores(event_store: RingBufferStore, connection_store: ConnectionStore) -> Self;
    pub fn connection_store(&self) -> Arc<RwLock<ConnectionStore>>;
}
```

`ApiState::new(event_store)` remains source-compatible and creates a default connection store.

## API Routes

Add `connections.rs`:

```rust
pub const CONNECTIONS_PATH: &str = "/api/v1/connections";
pub const CONNECTION_DETAIL_PATH: &str = "/api/v1/connections/{id}";
```

Routes:

- `GET /api/v1/connections`
- `GET /api/v1/connections/{id}`

Register routes in `server::router_with_state`.

## Query Contract

List endpoint supports:

```rust
pub struct ConnectionListQueryParams {
    pub limit: Option<usize>,
}
```

Limits:

- default `limit`: 100
- max `limit`: 500
- `limit = 0`: HTTP 400

No cursor/filter support in this task because Issue 030 only requires recent list and detail.

## Response Contract

List:

```rust
pub struct ConnectionListResponse {
    pub items: Vec<ConnectionResponse>,
}
```

Detail:

```rust
pub struct ConnectionResponse {
    pub id: String,
    pub protocol: String,
    pub database_type: String,
    pub client_addr: String,
    pub backend_addr: String,
    pub user: Option<String>,
    pub database: Option<String>,
    pub state: String,
    pub connected_at: String,
    pub closed_at: Option<String>,
    pub last_activity_at: Option<String>,
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub query_count: u64,
}
```

`ConnectionState` maps to snake_case:

- `Created` -> `created`
- `BackendConnected` -> `backend_connected`
- `HandshakeSeen` -> `handshake_seen`
- `Authenticating` -> `authenticating`
- `Ready` -> `ready`
- `CommandInFlight` -> `command_in_flight`
- `Closing` -> `closing`
- `Closed` -> `closed`
- `Failed` -> `failed`

## Error Contract

Missing connection:

- HTTP 404
- `NOT_FOUND`
- message: `Connection not found`
- details: `{ "id": "<requested-id>" }`

Invalid list limit:

- HTTP 400
- `BAD_REQUEST`
- details: `{ "field": "limit" }`

The response header still carries `x-request-id`.

## Compatibility

- `ApiState::new(event_store)` remains valid.
- `router()` remains valid and uses empty default event/connection stores.
- Existing SQL event and health endpoint behavior should not change.
