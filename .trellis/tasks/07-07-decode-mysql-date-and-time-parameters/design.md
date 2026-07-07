# Design

## Boundary

All implementation should stay inside `sql-lens-protocol-mysql`.

The existing `decode_parameters` function in `execute.rs` is the extension
point. Add temporal branches there instead of introducing a parallel decoder.

## Supported Type Codes

First scope:

- `DATE`
- `NEWDATE`
- `TIME`
- `DATETIME`
- `TIMESTAMP`

Defer fractional-second alternate type codes such as `TIME2`, `DATETIME2`, and
`TIMESTAMP2` unless packet evidence shows they are needed for client parameter
payloads in this milestone.

## Binary Value Shapes

Temporal values are length-prefixed binary values:

- `DATE`: length `0` or `4`
- `DATETIME` / `TIMESTAMP`: length `0`, `4`, `7`, or `11`
- `TIME`: length `0`, `8`, or `12`

For date/datetime:

- `year`: 2 bytes little-endian
- `month`: 1 byte
- `day`: 1 byte
- `hour`, `minute`, `second`: present when length is at least `7`
- `microseconds`: 4 bytes little-endian when length is `11`

For time:

- `is_negative`: 1 byte
- `days`: 4 bytes little-endian
- `hour`: 1 byte
- `minute`: 1 byte
- `second`: 1 byte
- `microseconds`: 4 bytes little-endian when length is `12`

## Representation

- `DATE` / `NEWDATE`: `SqlParameterValue::Date("YYYY-MM-DD")`.
- `DATETIME` / `TIMESTAMP`: `SqlParameterValue::Timestamp("YYYY-MM-DD HH:MM:SS")`.
- Temporal values with microseconds append `.ffffff`.
- `TIME`: `SqlParameterValue::Time("HH:MM:SS")` when days are zero and
  `SqlParameterValue::Time("DDD HH:MM:SS")` when days are non-zero.
- Negative time values are prefixed with `-`.
- Zero-length date/datetime/timestamp values use zero-form strings such as
  `0000-00-00` and `0000-00-00 00:00:00`.
- Zero-length time uses `00:00:00`.

## Error Handling

- Unsupported temporal length values should return a structured parse error.
- Truncated temporal payloads should return `IncompletePayload`.
- Adapter-level malformed temporal payloads should not update
  `last_client_command` or `last_statement_execute_envelope`.
- Do not log raw parameter payload bytes.

## Trade-Offs

- This layer formats strings but does not validate real calendar dates. MySQL
  can represent zero dates and application-specific edge cases that strict date
  libraries reject.
- Time zone conversion is excluded because MySQL binary parameter payloads do
  not carry enough context for SQL Lens to apply it safely.
