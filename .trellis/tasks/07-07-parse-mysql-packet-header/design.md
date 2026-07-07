# Parse MySQL packet header design

## Boundary

Implement in `crates/sql-lens-protocol-mysql`.

This task adds packet envelope header parsing only. It must not parse handshake payloads, commands, result packets, prepared statements, or emit SQL events.

## Protocol Reference

The MySQL client/server protocol packet header is four bytes:

```text
payload_length: int<3>
sequence_id:    int<1>
payload:        string<payload_length>
```

`payload_length` is little-endian and excludes the 4-byte header.

Primary reference: MySQL Server internals documentation, basic packets: `https://dev.mysql.com/doc/dev/mysql-server/latest/page_protocol_basic_packets.html`.

## Public API

Add `packet.rs`:

```rust
pub const MYSQL_PACKET_HEADER_LEN: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MysqlPacketHeader {
    pub payload_length: u32,
    pub sequence_id: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MysqlPacket<'a> {
    pub header: MysqlPacketHeader,
    pub payload: &'a [u8],
}

pub fn parse_mysql_packet(input: &[u8]) -> Result<MysqlPacket<'_>, MysqlPacketParseError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MysqlPacketParseError {
    IncompleteHeader { available: usize },
    IncompletePayload { declared: u32, available: usize },
}
```

Re-export from crate root:

```rust
pub use packet::{
    MYSQL_PACKET_HEADER_LEN, MysqlPacket, MysqlPacketHeader, MysqlPacketParseError,
    parse_mysql_packet,
};
```

## Parsing Rules

- If `input.len() < 4`, return `IncompleteHeader { available: input.len() }`.
- `payload_length = input[0] | input[1] << 8 | input[2] << 16`.
- `sequence_id = input[3]`.
- `available_payload = input.len() - 4`.
- If `available_payload < payload_length`, return `IncompletePayload`.
- Return a payload slice `&input[4..4 + payload_length]`.
- Ignore trailing bytes after the first complete packet for now. Future stream framing can repeatedly call the parser on the remaining bytes.

## Error Display

Implement `Display` and `Error` for `MysqlPacketParseError` with concise messages:

- `incomplete MySQL packet header: available N of 4 bytes`
- `incomplete MySQL packet payload: declared D bytes, available A bytes`

Do not add `thiserror` or `anyhow`.

## Tests

Unit tests in `packet.rs`:

- normal packet with payload length 3 and sequence id 2.
- empty payload packet with payload length 0.
- short header with 0..3 bytes.
- incomplete payload with declared length greater than available bytes.
- trailing bytes are not included in returned payload.

## Compatibility

This is additive to the MySQL protocol crate.

The existing `MysqlProtocolAdapter` no-op observation behavior should remain unchanged.

## Rollback

If packet stream handling complexity appears, pause before implementing it. This task owns only one-packet header parsing.
