# MySQL numeric prepared parameter research

## Sources

- MySQL source documentation: `COM_STMT_EXECUTE`, https://dev.mysql.com/doc/dev/mysql-server/latest/page_protocol_com_stmt_execute.html
- MySQL source documentation: field type constants, https://dev.mysql.com/doc/dev/mysql-server/latest/field__types_8h.html

## Notes

- Numeric parameter decoding requires the parameter type metadata section, not only the fixed execute envelope.
- `new_params_bind_flag = 1` means type metadata is present in the execute packet.
- `new_params_bind_flag = 0` means the server/client may rely on previously sent parameter type metadata.
- Supporting `new_params_bind_flag = 0` correctly requires storing parameter type metadata per prepared statement.
- Common numeric widths:
  - `TINY`: 1 byte.
  - `SHORT`: 2 bytes.
  - `LONG`: 4 bytes.
  - `LONGLONG`: 8 bytes.
  - `INT24`: 4 bytes.
  - `FLOAT`: 4 bytes.
  - `DOUBLE`: 8 bytes.
- Numeric values are little-endian.

## Recommended Scope

Support packets with `new_params_bind_flag = 1` first. Treat `new_params_bind_flag = 0` as a non-fatal unsupported branch until a later task introduces per-statement type metadata caching.
