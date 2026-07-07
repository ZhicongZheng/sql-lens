# Store prepared statement state per connection design

## Boundary

Implement in `crates/sql-lens-protocol-mysql`.

Do not change `sql-lens-core`, `sql-lens-protocol`, storage, API, WebSocket, UI, plugin, proxy, or app runtime code unless the open close-hook question is answered differently.

## State Shape

Recommended MySQL-local state:

```rust
pub struct MysqlPreparedStatement {
    pub statement_id: u32,
    pub template_sql: String,
    pub num_columns: u16,
    pub num_params: u16,
    pub warning_count: Option<u16>,
}
```

`MysqlConnectionState` should own:

```rust
prepared_statements: std::collections::BTreeMap<u32, MysqlPreparedStatement>
```

Use `BTreeMap` from the standard library for deterministic tests and no new dependencies. A `HashMap` would also work, but deterministic ordering is handy if future tests inspect all mappings.

Expose narrow read APIs:

```rust
impl MysqlConnectionState {
    pub fn prepared_statement(&self, statement_id: u32) -> Option<&MysqlPreparedStatement>;
    pub fn prepared_statement_count(&self) -> usize;
}
```

Avoid exposing mutable map access.

## Data Flow

1. Client sends `COM_STMT_PREPARE`.
2. Adapter stores `MysqlPendingStatementPrepare`.
3. Backend sends prepare OK.
4. Adapter stores `MysqlStatementPrepareOutcome` as today.
5. Adapter also inserts or replaces `prepared_statements[statement_id]` using:
   - `statement_id` from prepare OK.
   - `template_sql` from pending prepare command SQL.
   - `num_columns`, `num_params`, and `warning_count` from prepare OK.
6. Backend sends prepare ERR.
7. Adapter stores failed outcome but does not insert a mapping.

## Connection Close Semantics

Current evidence:

- `ProtocolAdapter` has `observe_client_bytes` and `observe_backend_bytes`.
- `ProtocolConnectionState` only supports downcasting with `as_any` / `as_any_mut`.
- There is no close callback.

Recommended scope: satisfy close cleanup through per-connection ownership. When the proxy drops a connection state, its prepared statement map drops with it. Explicit close hooks or durable lifecycle cleanup belong to a later shared protocol lifecycle task.

## Tests

- New state starts empty.
- Successful prepare OK inserts mapping.
- Failed prepare does not insert mapping.
- Reusing the same statement ID replaces the mapping.
- Separate connection states have isolated maps.
- Existing query tests continue to pass.

## Rollback

If the map insertion creates ambiguity with Issue 048 outcome state, keep the outcome state unchanged and make the map insertion a small private helper called only after successful prepare OK.
