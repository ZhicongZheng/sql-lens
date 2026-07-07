# Decode MySQL NULL bitmap design

## Boundary

Implement in `crates/sql-lens-protocol-mysql`.

Do not change core models, capture events, storage, API, WebSocket, frontend, plugin, proxy, or app runtime code.

## Protocol Shape

For `COM_STMT_EXECUTE`, the parameter payload after the fixed execute envelope begins with a NULL bitmap when the prepared statement has parameters.

The bitmap length is:

```rust
let null_bitmap_len = (parameter_count + 7) / 8;
```

Bits map to zero-based parameter indexes:

- byte `0`, bit `0` -> parameter `0`
- byte `0`, bit `1` -> parameter `1`
- byte `1`, bit `0` -> parameter `8`

Only parameter indexes `< parameter_count` are meaningful.

## Parser Shape

Recommended new MySQL-local module:

```rust
mod execute;
```

Recommended public parser contract:

```rust
pub struct MysqlNullBitmap {
    pub null_parameter_indexes: Vec<usize>,
    pub bytes_consumed: usize,
}

pub fn decode_null_bitmap(
    parameter_payload: &[u8],
    parameter_count: u16,
) -> Result<MysqlNullBitmap, MysqlExecuteParseError>;

pub enum MysqlExecuteParseError {
    IncompletePayload {
        field: &'static str,
        needed: usize,
        available: usize,
    },
}
```

Use `field: "null_bitmap"` for truncated bitmap bytes.

## State Shape

Extend MySQL-local execute envelope state:

```rust
pub struct MysqlStatementExecuteEnvelope {
    pub command: MysqlClientCommand,
    pub statement_id: u32,
    pub flags: u8,
    pub iteration_count: u32,
    pub has_parameter_payload: bool,
    pub statement: Option<MysqlPreparedStatement>,
    pub null_parameter_indexes: Vec<usize>,
}
```

Do not store raw bitmap bytes or raw parameter payload bytes.

## Adapter Behavior

- Before `Authenticated`: count bytes only and ignore execute, preserving Issue 050 behavior.
- After `Authenticated` with unknown statement ID:
  - store the execute envelope with `statement: None`.
  - set `null_parameter_indexes = Vec::new()`.
  - do not attempt bitmap decoding because `num_params` is unknown.
- After `Authenticated` with known statement ID:
  - derive bitmap length from `statement.num_params`.
  - decode NULL parameter indexes from bytes after the fixed execute envelope.
  - store decoded indexes on `last_statement_execute_envelope`.
- If known statement has `num_params = 0`, store an empty list and consume zero bitmap bytes.
- If the bitmap is truncated, keep adapter observation non-fatal and do not update execute state.
- Emit zero events.

## Compatibility

- Existing `COM_STMT_EXECUTE` envelope parsing remains unchanged.
- Existing query, prepare, prepared statement map, OK, and ERR behavior remains unchanged.
- No new dependencies.
- Parameter metadata and value decoding belong to later issues.

## Rollback

If adapter integration broadens the task too much, keep parser-level support and add adapter integration in the next task. Do not decode parameter types or values as a workaround.
