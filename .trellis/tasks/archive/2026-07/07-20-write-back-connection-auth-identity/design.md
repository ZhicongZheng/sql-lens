# Design: Connection Auth Identity Write-Back

## Boundaries

- `sql-lens-protocol-mysql`: copy handshake username/database into `MysqlConnectionState.connection`.
- `sql-lens-protocol`: optional neutral way for app to read updated connection snapshot from state.
- `sql-lens-proxy`: `ConnectionLifecycleRecord::set_session_identity(user, database)`.
- `sql-lens-app`: after observe (client and/or backend), if identity present, update lifecycle + upsert connection store.
- API/UI: no schema change; fields already exist.

## Data flow

```text
Client handshake packet
  -> MysqlConnectionState.observe_client_handshake_response
  -> connection.user / connection.database set
  -> later SqlEvent emission uses connection.*
App observe_client_bytes / observe_backend_bytes
  -> read connection snapshot from protocol state
  -> lifecycle.set_session_identity
  -> connection_store.upsert (non-blocking relative to forward write)
```

## Contracts

- Identity is optional; empty/missing stays `None`.
- Prefer not downcasting MySQL types in app if a trait method can expose `ConnectionInfo` snapshot.
- Do not put secrets on ConnectionInfo.

## Trade-offs

- Handshake-time identity on events: earlier correct SQL event fields without waiting for OK packet.
- Store upsert on identity change (not only on connect/finish) so live Connections UI updates while session is open.
