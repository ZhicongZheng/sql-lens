# Parse COM_STMT_EXECUTE envelope design

## Boundary

Implement in `crates/sql-lens-protocol-mysql`.

Do not change core models, capture events, storage, API, WebSocket, frontend, plugin, proxy, or app runtime code.

## Protocol Shape

Official MySQL protocol documentation describes `COM_STMT_EXECUTE` as a client command.

The envelope begins with:

- command byte `0x17`.
- little-endian `statement_id` as 4 bytes.
- one-byte flags.
- little-endian `iteration_count` as 4 bytes.

The remaining bytes depend on the prepared statement parameter count:

- NULL bitmap.
- `new_params_bind_flag`.
- parameter type metadata.
- parameter values.

This task parses only the envelope and a narrow marker indicating whether there are parameter payload bytes after the envelope. Detailed parsing belongs to later tasks.

## Parser Shape

Recommended parser types:

```rust
pub const MYSQL_COM_STMT_EXECUTE: u8 = 0x17;

pub struct MysqlComStmtExecute {
    pub statement_id: u32,
    pub flags: u8,
    pub iteration_count: u32,
    pub has_parameter_payload: bool,
}

pub enum MysqlParsedClientCommand {
    Query(MysqlComQuery),
    StatementPrepare(MysqlComStmtPrepare),
    StatementExecute(MysqlComStmtExecute),
}
```

Use the existing `parse_client_command` dispatch so unsupported command behavior stays centralized.

## State Shape

Recommended MySQL-local state:

```rust
pub struct MysqlStatementExecuteEnvelope {
    pub command: MysqlClientCommand,
    pub statement_id: u32,
    pub flags: u8,
    pub iteration_count: u32,
    pub has_parameter_payload: bool,
    pub statement: Option<MysqlPreparedStatement>,
}
```

Expose:

```rust
impl MysqlConnectionState {
    pub fn last_statement_execute_envelope(&self) -> Option<&MysqlStatementExecuteEnvelope>;
}
```

Use `statement: None` for unknown statement IDs.

## Adapter Behavior

- Before `Authenticated`: count bytes only and ignore execute.
- After `Authenticated`:
  - valid execute stores `last_client_command` with `kind = StatementExecute`, sequence ID, and an empty SQL string or future-safe command label.
  - valid execute stores `last_statement_execute_envelope`.
  - known statement ID clones the prepared statement metadata into the envelope.
  - unknown statement ID stores `statement = None`.
  - malformed execute packets are non-fatal and do not update execute state.
- Execute parsing emits zero events in this task.

## Compatibility

- Existing `COM_QUERY`, `COM_STMT_PREPARE`, prepare response, and prepared statement map behavior must remain unchanged.
- No new dependencies.
- Do not build parameter decoding abstractions before Issue 051+ need them.

## Rollback

If linking to prepared statement metadata broadens the task too much, keep parser-level support and adapter state with `statement = None` for all executes. Do not start parameter decoding to compensate.
