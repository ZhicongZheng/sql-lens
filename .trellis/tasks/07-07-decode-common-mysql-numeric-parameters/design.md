# Decode common MySQL numeric parameters design

## Boundary

Implement in `crates/sql-lens-protocol-mysql`.

Do not change storage, API, WebSocket, frontend, plugin, proxy, or app runtime code.

## Protocol Shape

After the fixed `COM_STMT_EXECUTE` envelope and NULL bitmap, the parameter payload includes:

- `new_params_bind_flag`
- parameter type metadata when the flag is `1`
- parameter values

Each parameter type metadata entry is two bytes:

- MySQL type code.
- flag byte, including unsigned marker for numeric types.

Numeric value byte widths are expected to be:

- `TINY`: 1 byte.
- `SHORT`: 2 bytes.
- `LONG`: 4 bytes.
- `LONGLONG`: 8 bytes.
- `INT24`: 4 bytes.
- `FLOAT`: 4 bytes.
- `DOUBLE`: 8 bytes.

All numeric values are little-endian in the binary protocol.

## Scope Decision

For Issue 052, support only packets with `new_params_bind_flag = 1`.

Packets with `new_params_bind_flag = 0` are non-fatal and do not update decoded numeric parameter state until a later task adds per-statement parameter type caching.

## Parser Shape

Recommended additions to `execute.rs`:

```rust
pub struct MysqlParameterType {
    pub type_code: u8,
    pub unsigned: bool,
}

pub struct MysqlDecodedParameter {
    pub index: u16,
    pub value: sql_lens_core::SqlParameterValue,
}

pub struct MysqlDecodedParameters {
    pub parameters: Vec<MysqlDecodedParameter>,
    pub bytes_consumed: usize,
}
```

Use structured `MysqlExecuteParseError::IncompletePayload { field, needed, available }` for truncated flags, metadata, or value bytes.

## State Shape

Recommended MySQL-local execute envelope extension:

```rust
pub struct MysqlStatementExecuteEnvelope {
    pub null_parameter_indexes: Vec<usize>,
    pub numeric_parameters: Vec<MysqlDecodedParameter>,
}
```

Use a MySQL-local decoded parameter type first because this task does not emit protocol-neutral events. Convert to core `SqlParameter` when event emission starts.

## Adapter Behavior

- Decode numeric parameters only when statement ID is known.
- Reuse `statement.num_params` and NULL bitmap output.
- For NULL indexes, store `SqlParameterValue::Null` and do not consume value bytes.
- For supported numeric type codes, consume fixed-width value bytes and store decoded values.
- For unsupported type codes, keep adapter observation non-fatal and do not update decoded numeric parameter state.
- For malformed metadata or values, keep observation non-fatal and do not update execute state.
- Emit zero events.

## Compatibility

- Existing `COM_STMT_EXECUTE` envelope and NULL bitmap behavior must remain unchanged.
- No new dependencies.
- Expanded SQL rendering stays in Issue 055.

## Rollback

If adapter integration broadens the task too much, implement parser-level numeric decoding first and leave adapter state integration for a follow-up task.
