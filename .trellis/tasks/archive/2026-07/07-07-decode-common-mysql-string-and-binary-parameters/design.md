# Design

## Boundary

All implementation stays inside `sql-lens-protocol-mysql`.

The shared `SqlParameterValue` enum already has enough variants for this task,
so decoded values can keep using the MySQL-local `MysqlDecodedParameter` wrapper
without changing protocol-neutral core contracts.

## Parser Shape

Rename the current numeric-only parser API to a general parameter decoder:

```rust
pub fn decode_parameters(
    parameter_payload_after_null_bitmap: &[u8],
    parameter_count: u16,
    null_parameter_indexes: &[usize],
) -> Result<Option<MysqlDecodedParameters>, MysqlExecuteParseError>
```

The function keeps the Issue 052 behavior:

- `parameter_count = 0` returns an empty decoded list.
- Missing `new_params_bind_flag` is an incomplete payload error.
- `new_params_bind_flag != 1` returns `Ok(None)`.
- Unsupported type codes return `Ok(None)` without partial decoded state.
- NULL bitmap indexes decode as `SqlParameterValue::Null` and consume no value
  bytes.

The existing `decode_numeric_parameters` name may remain as a compatibility
wrapper if that keeps the public crate surface less disruptive during the
incremental milestone.

## Supported Type Codes

Numeric types remain unchanged:

- `TINY`, `SHORT`, `LONG`, `LONGLONG`, `INT24`, `FLOAT`, `DOUBLE`.

Text types:

- `VARCHAR`
- `VAR_STRING`
- `STRING`
- `ENUM`
- `SET`

Binary summary types:

- `TINY_BLOB`
- `MEDIUM_BLOB`
- `LONG_BLOB`
- `BLOB`
- `BIT`
- `GEOMETRY`

## Length-Encoded Values

Text and binary values are read as MySQL length-encoded byte strings:

- first byte `< 0xfb`: one-byte length
- `0xfc`: next 2 bytes little-endian length
- `0xfd`: next 3 bytes little-endian length
- `0xfe`: next 8 bytes little-endian length

Truncated length prefixes or truncated value bytes return
`MysqlExecuteParseError::IncompletePayload`.

## Text Safety

Text decoding uses `String::from_utf8_lossy` so invalid UTF-8 is represented
with replacement characters instead of panicking or rejecting the whole execute
packet.

This is deliberately conservative. Later charset-aware decoding can use
parameter metadata once SQL Lens has a richer MySQL field metadata model.

## Binary Safety

Binary parameters produce summaries like:

```text
len=32 hex=00112233445566778899aabbccddeeff...
```

The summary includes:

- total byte length
- hex prefix of at most 16 bytes
- trailing `...` when the original value is longer than the prefix

The parser never stores raw binary bytes in connection state.

## Adapter State

Rename the execute envelope field from `numeric_parameters` to `parameters`.
The decoded list can now contain numeric values, strings, binary summaries, and
NULL values.

Unknown statement IDs still store an execute envelope with no decoded
parameters because SQL Lens cannot know the parameter count.

## Trade-Offs

- Text decoding is UTF-8-lossy rather than charset-aware. This satisfies the
  safety requirement now and leaves charset support for a later protocol
  metadata task.
- BLOB-family values are summarized even when an application uses them for text.
  Without charset metadata, treating them as binary is the safer default.
- JSON and decimal are not included in this task to keep scope aligned with
  Issue 053 and avoid broadening the value decoder before it is needed.
