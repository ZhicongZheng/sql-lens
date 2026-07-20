# Implement: Connection Auth Identity Write-Back

1. MySQL adapter: on successful client handshake parse, set `self.connection.user` / `database` from handshake.
2. Protocol trait: add `fn connection_info(&self) -> Option<&ConnectionInfo>` defaulting to `None`; implement on `MysqlConnectionState`.
3. Proxy lifecycle: `set_session_identity(user: Option<String>, database: Option<String>)` updating info + last_activity optional.
4. App `forward_protocol_connection` / observe helpers: after observe, sync identity from protocol state into lifecycle and upsert store when changed.
5. Tests: adapter unit tests for identity on connection + events; lifecycle unit test; runtime/API if cheap.
6. Validate: `cargo fmt`, `cargo test -p sql-lens-protocol-mysql -p sql-lens-proxy -p sql-lens-app --lib`, clippy on those packages.

## Rollback

Revert the four crates’ identity-related changes; no migration.
