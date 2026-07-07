# MySQL Packet Fixtures

These fixtures document MySQL-compatible packet framing test cases.

## Format

- Files use plain ASCII hex bytes.
- Bytes may be separated by spaces or newlines.
- Lines may contain comments after `#`.
- The bytes represent exactly the input passed to `parse_mysql_packet`.
- The first four bytes are the MySQL packet header:
  - bytes `0..3`: payload length as a 3-byte little-endian unsigned integer,
  - byte `3`: sequence ID.
- Remaining bytes are payload bytes.

The fixture tests cover only packet envelope parsing. They do not parse
handshake payloads, commands, result packets, prepared statements, or SQL text.

## Cases

| File | Expected result |
| --- | --- |
| `normal.hex` | payload length `3`, sequence ID `2`, payload `abc` |
| `empty-payload.hex` | payload length `0`, sequence ID `7`, empty payload |
| `malformed-short-header.hex` | `IncompleteHeader { available: 3 }` |
| `malformed-incomplete-payload.hex` | `IncompletePayload { declared: 5, available: 2 }` |
