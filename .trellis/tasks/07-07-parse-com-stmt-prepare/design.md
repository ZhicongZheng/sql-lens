# Parse COM_STMT_PREPARE design

## Boundary

Implement in `crates/sql-lens-protocol-mysql`.

Do not change core models, storage, API, WebSocket, proxy, frontend, or app runtime code. Prepared statement public event models will be used by later tasks after backend response parsing exists.

## Parser Shape

Extend the client command parser from one command to a small command enum.

Suggested public shape:

```rust
pub const MYSQL_COM_QUERY: u8 = 0x03;
pub const MYSQL_COM_STMT_PREPARE: u8 = 0x16;

pub enum MysqlCommandKind {
    Query,
    StatementPrepare,
}

pub struct MysqlComQuery {
    pub sql: String,
}

pub struct MysqlComStmtPrepare {
    pub template_sql: String,
}

pub enum MysqlParsedClientCommand {
    Query(MysqlComQuery),
    StatementPrepare(MysqlComStmtPrepare),
}

pub fn parse_client_command(
    payload: &[u8],
) -> Result<Option<MysqlParsedClientCommand>, MysqlCommandParseError>;
```

Keep UTF-8 decoding behavior consistent with `COM_QUERY`: invalid SQL/template bytes return `MysqlCommandParseError::InvalidUtf8`.

## State Shape

Add MySQL-local pending prepare state:

```rust
pub struct MysqlPendingStatementPrepare {
    pub command: MysqlClientCommand,
}

impl MysqlConnectionState {
    pub fn pending_statement_prepare(&self) -> Option<&MysqlPendingStatementPrepare>;
}
```

Reuse `MysqlClientCommand` with `kind = MysqlCommandKind::StatementPrepare`, packet sequence ID, and SQL template string in its `sql` field. Do not add a statement ID field yet.

## Adapter Behavior

- Before `Authenticated`: count bytes only and ignore `COM_STMT_PREPARE`.
- After `Authenticated`:
  - valid `COM_QUERY` behavior stays unchanged.
  - valid `COM_STMT_PREPARE` stores `last_client_command` and `pending_statement_prepare`.
  - invalid UTF-8 or malformed command stays non-fatal and does not update command state.
  - unsupported commands stay non-fatal.
- `COM_STMT_PREPARE` emits zero events in this task.
- Starting a new prepare command replaces any existing pending prepare state.

## Tests

Parser tests:

- Valid `COM_STMT_PREPARE` fixture extracts SQL template.
- Empty template is accepted.
- Invalid UTF-8 template returns `InvalidUtf8`.
- Existing `COM_QUERY` parser tests still pass.

Adapter tests:

- `COM_STMT_PREPARE` before auth does not update command or pending prepare state.
- `COM_STMT_PREPARE` after auth stores kind, sequence ID, and SQL template.
- Unsupported command behavior is unchanged.
- `COM_QUERY` pending query behavior is unchanged.

## Rollback

If the parser enum refactor becomes too broad, add a separate `parse_com_stmt_prepare` helper and dispatch in the adapter. Do not parse backend prepare response in this task to compensate.
