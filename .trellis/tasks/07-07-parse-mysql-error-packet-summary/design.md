# Parse MySQL error packet summary design

## Boundary

Implement in `crates/sql-lens-protocol-mysql`.

Do not change storage, API, WebSocket, proxy, frontend, or app runtime code. Use the existing protocol-neutral `ErrorSummary` model from `sql-lens-core`.

## Parser Shape

Add a MySQL-local ERR packet parser, preferably in `error.rs` or `err.rs`:

```rust
pub struct MysqlErrPacketSummary {
    pub error_code: u16,
    pub sql_state: Option<String>,
    pub message: String,
}

pub fn parse_err_packet_summary(
    payload: &[u8],
) -> Result<Option<MysqlErrPacketSummary>, MysqlErrPacketParseError>;
```

`Ok(None)` means the payload is not an ERR packet. Parser errors mean the payload looked like an ERR packet but did not contain the minimum required fields.

## Field Parsing

Payload layout:

```text
0xff
error_code: int<2> little endian
if protocol 41 shape is present:
  sql_state_marker: string[1]
  sql_state: string[5]
error_message: string<EOF>
```

Parsing rules:

- Empty payload returns an incomplete header error.
- Header other than `0xff` returns `Ok(None)`.
- Missing error code returns an incomplete error-code error.
- SQLSTATE is present only when the remaining payload starts with `#` and has at least six bytes.
- Message bytes should be decoded with `String::from_utf8_lossy`.
- Message sanitization should remove or replace ASCII control characters except tab/newline/carriage return if we choose to preserve readable whitespace.

## Event Integration

For ERR finalization:

1. Parse ERR packet summary from backend payload.
2. Build the failed `SqlEvent` as before.
3. If parsing succeeds with `Some(summary)`, set:
   - `event.error = Some(ErrorSummary { code, sql_state, message, metadata })`
   - `event.result = None`
4. If parsing returns `Ok(None)` or an error, keep finalization non-fatal and leave `event.error = None`.

OK finalization should stay unchanged.

## Metadata

Use `ErrorSummary.metadata` for MySQL-only fields:

- `mysql_error_code` as `MetadataValue::Unsigned(u64::from(error_code))`

Use `ErrorSummary.code = Some(error_code.to_string())` so UI and API consumers can display the vendor code without parsing metadata.

## Tests

Parser tests:

- Official-style ERR fixture with error code `1096`, SQLSTATE `HY000`, and message `No tables used`.
- ERR fixture without SQLSTATE.
- Incomplete error-code payload returns a structured error.
- Non-ERR payload returns `Ok(None)`.
- Message with control characters is sanitized.

Adapter tests:

- Backend ERR finalization populates `ErrorSummary`.
- Error summary metadata includes MySQL error code.
- Malformed ERR summary still finalizes as error with `error = None`.
- OK event result summary behavior remains unchanged.

## Rollback

If error message sanitization policy becomes contentious, keep code and SQLSTATE parsing and use a minimal control-character replacement policy. Do not add a broad redaction engine in this task.
