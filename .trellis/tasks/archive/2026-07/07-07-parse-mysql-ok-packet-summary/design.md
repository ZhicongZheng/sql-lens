# Parse MySQL OK packet summary design

## Boundary

Implement in `crates/sql-lens-protocol-mysql`.

Do not change `sql-lens-core` models, storage, API, WebSocket, proxy, or frontend code. `ResultSummary` already has the fields needed for affected rows.

## Parser Shape

Add a MySQL-local OK packet parser, preferably in a small `ok.rs` module:

```rust
pub struct MysqlOkPacketSummary {
    pub affected_rows: u64,
    pub status_flags: Option<u16>,
}

pub fn parse_ok_packet_summary(
    payload: &[u8],
) -> Result<Option<MysqlOkPacketSummary>, MysqlOkPacketParseError>;
```

`Ok(None)` means the payload is not a command OK packet for this task. Parser errors mean the payload looked like an OK packet but did not have enough bytes to decode the requested fields.

## Length-Encoded Integer

Add a private helper in the OK parser module:

```rust
fn read_lenenc_integer(input: &[u8]) -> Result<(u64, usize), MysqlOkPacketParseError>;
```

Supported forms:

- First byte `< 0xfb`: value is that byte, consumed length is 1.
- First byte `0xfc`: read 2 little-endian bytes.
- First byte `0xfd`: read 3 little-endian bytes.
- First byte `0xfe`: read 8 little-endian bytes.
- First byte `0xfb`: treat as unsupported or invalid for OK packet integer fields.

Do not introduce a generic MySQL codec abstraction yet.

## Adapter Integration

Current query finalization detects OK by payload first byte `0x00`.

For OK finalization:

1. Parse the OK packet summary from the backend payload.
2. Build `SqlEvent` as before.
3. If parsing succeeds with `Some(summary)`, set:
   - `event.result = Some(ResultSummary { affected_rows: Some(summary.affected_rows), returned_rows: None })`
   - add MySQL metadata `ok_status_flags = summary.status_flags` when present.
4. If parsing returns `Ok(None)` or an error, keep event finalization non-fatal and leave `event.result = None`.

ERR finalization must stay unchanged.

## Metadata

Keep MySQL-only fields in `ProtocolMetadata.fields`.

Existing fields:

- `command = "COM_QUERY"`
- `command_sequence_id`

Add when available:

- `ok_status_flags` as `MetadataValue::Unsigned(u64::from(status_flags))`

Do not add status flags to `SqlEvent` or `ResultSummary`.

## Tests

Parser tests:

- Official-style OK fixture with affected rows `0`, last insert ID `0`, status flags `0x0002`.
- OK fixture with affected rows above the one-byte length-encoded range.
- Incomplete length-encoded integer errors.
- Non-OK payload returns `Ok(None)`.

Adapter tests:

- Backend OK finalization populates `ResultSummary.affected_rows`.
- OK status flags are present in MySQL metadata.
- Malformed OK summary still finalizes as OK with `result = None`.
- ERR finalization remains `result = None`.

## Rollback

If OK summary parsing causes ambiguity with result-set packets, keep the standalone parser and defer adapter integration. Do not broaden the task into result-set lifecycle parsing.
