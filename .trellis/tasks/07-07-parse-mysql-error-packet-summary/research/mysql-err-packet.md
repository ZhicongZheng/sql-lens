# MySQL ERR Packet Research

## Sources

- MySQL source documentation: `ERR_Packet`, https://dev.mysql.com/doc/dev/mysql-server/latest/page_protocol_basic_err_packet.html

## Notes

- A MySQL ERR packet signals that an error occurred.
- The payload starts with header `0xff`.
- The next field is a 2-byte little-endian error code.
- When `CLIENT_PROTOCOL_41` is enabled, the packet includes:
  - a one-byte SQL state marker, usually `#`
  - a five-byte SQLSTATE value
- The remaining payload bytes are the human-readable error message.
- The official example packet contains:
  - header `0xff`
  - error code `0x0448` (`1096`)
  - SQL state marker `#`
  - SQLSTATE `HY000`
  - message `No tables used`

## Task Scope Decision

This task should parse command ERR packets and attach a sanitized protocol-neutral `ErrorSummary` to failed query events. It should not implement general redaction rules, error-code classification, or authentication behavior changes.
