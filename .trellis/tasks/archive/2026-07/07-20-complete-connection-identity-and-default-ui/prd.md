# Complete Connection Identity And Default UI Delivery

## Goal

Close two core v1 product gaps so a developer can inspect **who** connected and open the **local dashboard from one `sql-lens` process** without a separate Vite server.

## Background (confirmed)

- Parent is planning/integration only; work ships through ordered children:
  1. `07-20-write-back-connection-auth-identity`
  2. `07-20-default-static-ui-delivery`
- `ConnectionLifecycleRecord::accepted` always sets `user: None`, `database: None` and has no identity update API (`crates/sql-lens-proxy/src/lifecycle.rs`).
- MySQL adapter parses client handshake `username` / `database` into `MysqlClientHandshakeResponse` and tracks auth phase, but never copies them into `MysqlConnectionState.connection` (`crates/sql-lens-protocol-mysql/src/lib.rs`).
- Emitted `SqlEvent`s therefore also carry empty `user`/`database` even after successful login.
- Web Connections UI already renders `connection.user` and `connection.database` columns.
- HTTP server already serves SPA via optional `web.static_dir` (`crates/sql-lens-api/src/server.rs`); default is `None`. Docs mention example paths but do not define a package-default delivery story.
- Frontend builds with `npm run build` → `crates/sql-lens-app/web/dist`.

## Requirements

1. After MySQL-compatible authentication identity is known, connection records exposed by Connections API/UI include `user` and `database` when the client supplied them.
2. Captured SQL events for that connection also carry the same `user`/`database`.
3. Default/local developer delivery can serve the built web UI from the SQL Lens HTTP listener via `web.static_dir`, with documented build + config steps (and a sensible default path where appropriate).
4. Preserve packet-forwarding isolation: identity and static UI work must not block TCP I/O on storage or plugins.

## Acceptance Criteria

- [ ] Child 1: connection list/detail show handshake username/database for authenticated MySQL-compatible sessions (when present on the wire).
- [ ] Child 1: SQL events for those sessions include matching `user`/`database`.
- [ ] Child 1: failed/rejected connections without identity remain valid with `None` fields.
- [ ] Child 2: building the web app produces a directory that `web.static_dir` can serve; same listener serves API + SPA routes.
- [ ] Child 2: README/CONFIG document the default delivery path and build command.
- [ ] Workspace validation for touched packages passes after both children land.

## Out Of Scope

- Plugin/exporter work.
- PostgreSQL / TLS / DuckDB.
- UI authentication / RBAC.
- Changing MySQL auth plugin cryptography or multi-factor auth protocol support.
- Hot-reload of config without restart.

## Task Map

| Child | Deliverable |
|-------|-------------|
| `07-20-write-back-connection-auth-identity` | Protocol + lifecycle + runtime write-back for user/database |
| `07-20-default-static-ui-delivery` | Build/docs/default static_dir delivery for web UI |

## Decisions

- Identity is **database session** user/database from the wire (not SQL Lens UI login).
- Copy into adapter `ConnectionInfo` at client handshake; upsert connection store when identity becomes known (handshake / auth path).
