# Write Back Connection Auth Identity

## Goal

Populate `ConnectionInfo.user` and `ConnectionInfo.database` from MySQL-compatible login observation so Connections API/UI and SQL events show session identity.

## Background

- Handshake parser already extracts `username` / `database`.
- `MysqlConnectionState` keeps `client_handshake` separately; `connection.user` / `connection.database` stay `None` from app lifecycle.
- App records connections via lifecycle only — protocol identity never flows back.
- UI already displays the fields.
- Product is local developer use (no UI login); identity here means **database session user/database from the wire**, not SQL Lens product auth.

## Decision

- Copy handshake `username`/`database` into the adapter’s protocol-neutral `ConnectionInfo` when the client handshake is parsed (so subsequent SQL events carry identity).
- Upsert the app connection store when auth succeeds **or** when identity first becomes known on the adapter snapshot (prefer upsert once identity is non-empty after handshake; re-upsert on auth success is fine if already set).
- Failed dial / limit rejects may keep `None`.

## Requirements

1. Adapter stores handshake username/database on `ConnectionInfo` used for SQL event emission.
2. App connection store reflects the same identity for live/closed sessions when known.
3. Protocol-neutral surface: lifecycle may gain a small identity setter; app composes without MySQL packet parsing.
4. No auth secrets, scramble data, or raw handshake bytes.
5. Missing username/database remains `None`.

## Acceptance Criteria

- [ ] Adapter tests: after client handshake, `connection().user` / `database` match fixtures; SQL events match.
- [ ] Runtime/API path: connection list/detail returns user/database for successful proxied login that sent them.
- [ ] Failed dial / connection-limit rejects still work with identity unset.
- [ ] No password/auth response bytes in logs or connection models.
- [ ] Touched-crate tests + fmt/clippy pass.

## Out Of Scope

- COM_CHANGE_USER mid-session (unless trivial).
- Non-MySQL protocols.
- UI redesign.
- SQL Lens product/UI authentication.
