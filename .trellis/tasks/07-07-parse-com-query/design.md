# Parse COM_QUERY design

## Boundary

Implement in `crates/sql-lens-protocol-mysql`.

This task adds MySQL client command parsing after authentication. It must not emit SQL events, measure timing, parse backend responses, update protocol-neutral core models, or add capture/storage/API dependencies.

## Current State

- `parse_mysql_packet` parses one complete MySQL packet.
- `MysqlConnectionState` can reach `Authenticated` after an OK auth result.
- `observe_client_bytes` currently only observes client handshake response while in `InitialHandshakeSeen`.
- No MySQL command parser exists yet.

## Packet Shape

MySQL commands use the first payload byte as the command code.

```text
COM_QUERY:
  command: 0x03
  sql: string<EOF>
```

The packet envelope sequence ID is outside the command payload, but storing it in MySQL state is useful for later timing/debugging tasks.

## Public API

Add a command parser module:

```rust
pub enum MysqlCommandKind {
    Query,
}

pub struct MysqlClientCommand {
    pub kind: MysqlCommandKind,
    pub sequence_id: u8,
    pub sql: String,
}

pub struct MysqlComQuery {
    pub sql: String,
}

pub fn parse_client_command(
    payload: &[u8],
) -> Result<Option<MysqlComQuery>, MysqlCommandParseError>;
```

If implementation clarity benefits from naming the command byte separately, expose:

```rust
pub const MYSQL_COM_QUERY: u8 = 0x03;
```

Extend MySQL state:

```rust
impl MysqlConnectionState {
    pub fn last_client_command(&self) -> Option<&MysqlClientCommand>;
}
```

## Parser Behavior

- Empty payload returns `IncompletePayload { field: "command" }`.
- First byte `0x03` parses the remaining payload as UTF-8 SQL text.
- Invalid UTF-8 returns `InvalidUtf8 { field: "sql" }`.
- Other command bytes return `Ok(None)` so unsupported commands remain non-fatal.
- The parser returns owned SQL text and does not store raw packet bytes.

Empty SQL text should parse successfully as `COM_QUERY` with `sql == ""`. MySQL servers may reject it later; this parser only records what the application sent.

## Adapter Behavior

`observe_client_bytes` should:

- Continue existing client handshake behavior while phase is `InitialHandshakeSeen`.
- Attempt command parsing only when phase is `Authenticated`.
- Parse a complete MySQL packet using `parse_mysql_packet`.
- On supported `COM_QUERY`, store `MysqlClientCommand` with command kind, sequence ID, and SQL text.
- On unsupported command, malformed packet, or invalid UTF-8, keep existing state and return `ProtocolObservation::new(bytes.len(), 0)`.
- Emit zero events.

## Compatibility

This is additive inside `sql-lens-protocol-mysql`.

Later Issue 044 can use `last_client_command()` or replace it with a pending-command slot when timing is added. Later character-set support can refine SQL decoding while preserving the same non-fatal adapter behavior.

## Tests

Parser tests:

- Parses `COM_QUERY` SQL text.
- Parses empty SQL text.
- Returns `None` for unsupported command byte.
- Rejects empty payload.
- Rejects invalid UTF-8 SQL text.

Adapter tests:

- `COM_QUERY` before authentication does not update command state.
- `COM_QUERY` after authentication stores command kind, sequence ID, and SQL text.
- Unsupported command after authentication is non-fatal and non-transitioning.
- Invalid UTF-8 after authentication is non-fatal.
- Command observation emits zero events.

## Rollback

If state retention proves premature, keep the standalone parser and remove `last_client_command()` from state. Do not add timing or event emission as a workaround in this task.
