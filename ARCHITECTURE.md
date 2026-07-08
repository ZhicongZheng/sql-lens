# SQL Lens Architecture

## Overview

SQL Lens is a transparent SQL debug proxy. It accepts database client connections, forwards traffic to a backend database, decodes protocol traffic when supported, emits normalized capture events, stores recent events, and serves them through REST, WebSocket, and a web UI.

```text
Application
    |
    | database protocol
    v
+-------------------+
| SQL Lens Proxy    |
| listener/session  |
+---------+---------+
          |
          v
+-------------------+       +-------------------+
| Protocol Adapter  |<----->| Backend Database  |
| mysql, future pg  |       | MySQL, Doris, ... |
+---------+---------+       +-------------------+
          |
          v
+-------------------+
| Capture Pipeline  |
+----+---------+----+
     |         |
     v         v
 Storage   WebSocket
     |
     v
 REST API
     |
     v
 Web UI
```

## Design Goals

- Forward traffic without requiring application code changes.
- Keep packet forwarding independent from event capture.
- Keep protocol-specific parsing inside protocol adapters.
- Use a shared SQL capture model across all protocols.
- Make storage pluggable.
- Keep the default deployment local and low-resource.
- Build every module so it can be tested in isolation.

## Data Flow

1. The client opens a connection to SQL Lens.
2. SQL Lens opens a backend connection to the configured database.
3. The proxy creates a connection session.
4. Bytes flow in both directions.
5. The protocol adapter observes packets and updates protocol state.
6. When a SQL execution is identified, the adapter emits a normalized event.
7. The capture pipeline enriches the event with timing, connection, and policy data.
8. Storage writes the event to the ring buffer and optional persistent backend.
9. The WebSocket broadcaster pushes the event to subscribed UI clients.
10. REST APIs query storage for list, detail, connections, and statistics.

## Network Model

SQL Lens runs separate listeners for proxy traffic and web/API traffic.

Recommended defaults:

- Proxy listener: `127.0.0.1:3307`.
- Web/API listener: `127.0.0.1:5173`.
- Backend database: configured by `backend.address`.

Connection model:

- One client TCP connection maps to one backend TCP connection for v1.
- SQL Lens does not pool backend connections in v1.
- SQL Lens does not multiplex multiple client sessions over one backend connection.
- TLS passthrough and TLS termination are separate configuration modes.

Multi-target model:

- One SQL Lens process may run multiple explicitly configured proxy targets.
- Each target maps one listener address to one backend database address.
- All target listeners share one API/storage/broadcast runtime state.
- Target identity is protocol-neutral and should be carried into captured
  connection/event data for API and UI display.
- SQL Lens does not dynamically route one listener to multiple backends based on
  SQL text, username, database name, SNI, or packet contents.
- Multi-target support must not introduce sharding, read/write splitting,
  failover, load balancing, SQL rewrite, or traffic policy ownership.

## Threading Model

Rust implementation should use Tokio.

Recommended runtime:

- One Tokio multi-thread runtime for proxy and API services.
- One task per accepted proxy connection.
- Two directional copy loops per connection.
- One protocol state machine per connection.
- One capture pipeline handle shared across sessions.
- One broadcaster for live WebSocket subscribers.

CPU-heavy work such as SQL formatting, fingerprinting, or large payload redaction should be bounded and moved away from hot forwarding paths if needed.

## Async Model

The proxy path must remain non-blocking.

Core async components:

- TCP accept loop.
- Client-to-backend read/write loop.
- Backend-to-client read/write loop.
- Protocol parser observer.
- Capture event channel.
- Storage writer.
- WebSocket broadcaster.
- REST handlers.

Backpressure rules:

- Packet forwarding should not wait for the UI.
- Capture events may be dropped according to configured policy if the capture channel is full.
- Dropped capture events must increment internal counters.
- Storage writes should be bounded.

## Module Responsibilities

### `sql-lens-core`

Shared domain model:

- `SqlEvent`.
- `ConnectionInfo`.
- `PreparedStatementInfo`.
- `SqlParameter`.
- `QueryTiming`.
- `CaptureStatus`.
- `ProtocolMetadata`.
- Error and result summary types.

This crate must not depend on protocol-specific crates.

### `sql-lens-config`

Startup configuration contract:

- Top-level configuration model.
- Section-specific configuration structs.
- Configuration option enums.
- Default values.

Config file loading, environment overrides, validation, and runtime apply logic belong outside this crate.

### `sql-lens-capture`

Capture pipeline primitives:

- Bounded capture event channel.
- Non-blocking event publisher.
- Capture event receiver for future fan-out.
- Overload policy.
- Dropped-event counters.

This crate should not parse protocol packets, persist events, broadcast WebSocket messages, or block packet forwarding.

### `sql-lens-proxy`

Network proxy:

- Listener.
- Session lifecycle.
- Backend dialing.
- Bidirectional forwarding.
- Capture hooks.
- TLS mode coordination.

This crate should not contain SQL rendering logic.

### `sql-lens-protocol`

Protocol adapter traits:

- Adapter registry.
- Object-safe packet observer trait.
- Type-erased connection state trait.
- Event emission contract.

The protocol adapter contract emits normalized `SqlEvent` values into an abstract event sink. Runtime capture channel publishing is composed outside this crate so protocol parsing stays independent from channel overload policy.

### `sql-lens-protocol-mysql`

MySQL-compatible protocol implementation:

- Packet framing.
- Handshake tracking.
- Authentication tracking.
- Command parsing.
- Prepared statement tracking.
- Parameter decoding.
- Error packet parsing.

### `sql-lens-storage`

Storage interface and implementations:

- Ring buffer.
- SQLite.
- Future DuckDB.
- Retention.
- Query filters.
- Aggregations.

### `sql-lens-api`

HTTP and WebSocket API:

- REST handlers.
- WebSocket subscriptions.
- API schemas.
- Error response mapping.

### `sql-lens-plugin`

Extension contracts:

- Hook traits.
- Exporter traits.
- Plugin context.
- Plugin error isolation.

### `sql-lens-app`

Binary composition:

- CLI.
- Config loading.
- Service startup.
- Shutdown.
- Logging.

## Proxy Lifecycle

```text
Configured
  -> Starting
  -> Listening
  -> AcceptingConnection
  -> DialingBackend
  -> Handshaking
  -> Authenticated
  -> Forwarding
  -> Draining
  -> Closed
```

Failure paths:

- Listener bind failure.
- Backend dial failure.
- Authentication failure.
- Protocol parse failure.
- Client disconnect.
- Backend disconnect.
- Shutdown signal.

## Connection Lifecycle

Each connection has:

- Stable connection ID.
- Client address.
- Backend address.
- Protocol.
- User when known.
- Database when known.
- Connection state.
- Bytes in and out.
- Query count.
- Last activity.
- Open prepared statements.

The connection record exists even if the protocol handshake fails.

## Prepared Statement Lifecycle

MySQL-compatible lifecycle:

```text
COM_STMT_PREPARE(sql)
  -> database response(statement_id, parameter_count, column_count)
  -> store statement_id -> sql template in connection state
  -> COM_STMT_EXECUTE(statement_id, parameters)
  -> decode parameter values
  -> render expanded SQL
  -> capture event
  -> COM_STMT_CLOSE(statement_id)
  -> remove statement from connection state
```

Rules:

- Statement IDs are scoped to one connection.
- Unknown statement IDs should create a capture event with degraded metadata.
- Large or binary parameters should use summaries, not full values, unless explicitly configured.
- Expansion is for display and replay preparation, not for modifying forwarded traffic.

## WebSocket Push Flow

1. UI connects to `/ws/sql`.
2. API authenticates the WebSocket request if auth is enabled.
3. Client sends subscription filters.
4. Capture pipeline broadcasts new events.
5. WebSocket layer applies filters.
6. Matching events are serialized as JSON.
7. Client receives event, updates cache, and renders live list.

WebSocket events must include a `type`, `version`, and `payload`.

## API Flow

REST requests use storage queries:

```text
UI -> API handler -> query parser -> storage query -> response schema -> UI
```

REST handlers should not inspect protocol internals directly. Protocol-specific metadata is returned as structured JSON under `metadata`.

## Storage Model

Default storage is an in-memory ring buffer.

Optional storage:

- SQLite for local persistence.
- DuckDB later for analytical workloads.

Storage is append-oriented for capture events. Updates are allowed for connection state and derived statistics, but captured SQL events should be immutable after finalization.

## Shutdown

Shutdown order:

1. Stop accepting new proxy connections.
2. Stop accepting new API connections.
3. Notify active sessions.
4. Drain capture channel within timeout.
5. Flush persistent storage.
6. Close WebSocket subscribers.
7. Exit.

## Operational Boundaries

SQL Lens should be safe and predictable for local development. It should not claim production proxy guarantees until the project has explicit hardening for:

- High availability.
- Connection pooling.
- TLS lifecycle.
- Credential handling.
- Long-term retention.
- Multi-user access control.
- Resource isolation.
