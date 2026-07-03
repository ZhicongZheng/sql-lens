# SQL Lens Protocol Design

## Overview

SQL Lens is a multi-protocol SQL debug proxy. Protocol support is implemented through adapters that translate database-specific traffic into a shared SQL capture model.

The v1 protocol target is MySQL-compatible databases. Future adapters should support PostgreSQL, ClickHouse, and feasible SQLite integration paths without changing the core event model.

## Protocol Adapter Contract

Each protocol adapter owns:

- Handshake observation.
- Authentication observation.
- Packet framing.
- Command parsing.
- Query extraction.
- Prepared statement lifecycle tracking.
- Parameter decoding.
- Error mapping.
- Result summary extraction.
- Protocol-specific metadata.

Each adapter emits shared `SqlEvent` records and may attach protocol-specific fields under `metadata`.

## MySQL-Compatible Protocol Support

Initial databases:

- MySQL.
- StarRocks.
- TiDB.
- Apache Doris.

Initial commands:

- `COM_QUERY`.
- `COM_STMT_PREPARE`.
- `COM_STMT_EXECUTE`.
- `COM_STMT_CLOSE`.
- `COM_PING`.
- `COM_QUIT`.

Observed but not deeply decoded in v1:

- Result set columns.
- OK packets.
- EOF packets.
- Error packets.

Deferred:

- `COM_CHANGE_USER`.
- `COM_INIT_DB`.
- `COM_FIELD_LIST`.
- `COM_STMT_SEND_LONG_DATA`.
- `COM_STMT_RESET`.
- Compression.
- Replication protocol.
- Load data local infile.
- Multi-result advanced parsing.

## MySQL Packet Basics

A MySQL packet has:

- 3-byte payload length.
- 1-byte sequence ID.
- Payload.

SQL Lens must preserve packet bytes while observing enough structure to reconstruct SQL events.

Sequence IDs are scoped to a command exchange. The adapter should detect malformed or unexpected sequence transitions for diagnostics, but forwarding should remain best-effort unless a safety rule requires closing the session.

## Authentication

SQL Lens should forward authentication traffic by default.

Requirements:

- Do not log passwords or password responses.
- Do not persist authentication packet payloads.
- Record authentication state transitions only.
- Support common authentication plugins through forwarding even when not decoded.
- Keep TLS mode explicit in configuration.

Authentication state:

```text
InitialHandshake
  -> ClientHandshakeResponse
  -> AuthSwitchOrResult
  -> Authenticated | AuthFailed
```

## Command Handling

### `COM_QUERY`

The payload contains SQL text.

Capture:

- Original SQL.
- Normalized SQL when available.
- Timing.
- Status.
- Error packet details when available.

### `COM_STMT_PREPARE`

The payload contains the SQL template.

Capture:

- Statement prepare event.
- Statement ID after backend response.
- Parameter count.
- Column count.

Connection state:

- Map `statement_id` to SQL template.

### `COM_STMT_EXECUTE`

The payload contains:

- Statement ID.
- Flags.
- Iteration count.
- NULL bitmap.
- New parameter bound flag.
- Parameter type metadata.
- Parameter values.

Capture:

- Statement ID.
- SQL template.
- Parameter list.
- Expanded SQL.
- Timing.
- Result or error summary.

### `COM_STMT_CLOSE`

Remove statement state for the connection.

### `COM_PING`

Capture as connection activity, not a SQL event unless diagnostic capture is enabled.

### `COM_QUIT`

Mark connection as closing.

## Prepared Statement Expansion

Expansion is a display transformation. It must never alter forwarded traffic.

Rules:

- Preserve original template.
- Preserve typed parameter list.
- Render expanded SQL using SQL literal escaping rules.
- Show unsupported parameters as placeholders with metadata.
- Show binary values as summaries by default.
- Apply redaction before storage and UI display when configured.

Supported parameter classes:

- NULL.
- Signed and unsigned integers.
- Floating point.
- Decimal as string when exact decoding is uncertain.
- Boolean.
- String.
- Date.
- Time.
- Timestamp.
- JSON.
- Binary summary.

## Protocol State Machine

Connection-level state:

```text
Created
  -> BackendConnected
  -> HandshakeSeen
  -> Authenticating
  -> Ready
  -> CommandInFlight
  -> Ready
  -> Closing
  -> Closed
```

Command-level state:

```text
Idle
  -> ClientCommandSeen
  -> BackendResponseInProgress
  -> Complete | Failed | Unknown
```

Prepared statement state:

```text
Unknown
  -> Preparing
  -> Prepared
  -> Executing
  -> Prepared
  -> Closed
```

## Connection Scope

Protocol state is scoped to one client/backend connection pair.

Do not share prepared statement IDs across connections.

If a connection is reset, all protocol state is discarded.

## Error Mapping

Error events should include:

- Protocol.
- Vendor error code when available.
- SQLSTATE when available.
- Sanitized message.
- Command type.
- Statement ID when relevant.
- Raw protocol metadata only when safe.

## Future PostgreSQL Design

PostgreSQL support should use a separate adapter.

Important concepts:

- Startup message.
- Authentication request/response.
- Simple Query protocol.
- Extended Query protocol.
- Parse.
- Bind.
- Describe.
- Execute.
- Sync.
- ReadyForQuery.
- ErrorResponse.

Prepared statement expansion maps naturally from Parse and Bind messages, but portal and statement scoping differ from MySQL. The shared capture model should not assume numeric MySQL statement IDs.

Recommended shared fields:

- `statement_key`.
- `statement_name`.
- `portal_name`.
- `parameter_formats`.
- `result_formats`.
- `metadata.postgresql`.

## Future SQLite Design

SQLite is embedded, so it does not normally expose a TCP protocol to proxy.

Potential support paths:

- Driver shim.
- LD_PRELOAD or dynamic library interception where practical.
- Application-side tracing adapter.
- Log ingestion.

SQLite support should be treated as a SQL execution surface, not forced into the TCP proxy model.

## Future ClickHouse Design

ClickHouse may be supported through:

- Native protocol adapter.
- HTTP SQL interface adapter.

The core capture model should support both request/response SQL surfaces and binary protocol surfaces.

