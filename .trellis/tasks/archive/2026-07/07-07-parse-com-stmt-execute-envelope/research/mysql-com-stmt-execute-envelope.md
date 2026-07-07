# MySQL COM_STMT_EXECUTE envelope research

## Sources

- MySQL source documentation: `COM_STMT_EXECUTE`, https://dev.mysql.com/doc/dev/mysql-server/latest/page_protocol_com_stmt_execute.html

## Notes

- `COM_STMT_EXECUTE` executes a server-side prepared statement.
- The command byte is `0x17`.
- The envelope includes a 4-byte little-endian statement ID.
- The envelope includes a one-byte flags field.
- The envelope includes a 4-byte little-endian iteration count.
- Remaining bytes depend on statement parameter count and include NULL bitmap, new-parameter-bound flag, parameter type metadata, and values.
- NULL bitmap and parameter value parsing belong to later tasks.

## Task Scope Decision

This task should parse only the execute envelope and perform a connection-local statement lookup when possible. Unknown statement IDs should remain non-fatal if the user accepts the recommended scope.
