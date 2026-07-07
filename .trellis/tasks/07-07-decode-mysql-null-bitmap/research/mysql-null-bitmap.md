# MySQL COM_STMT_EXECUTE NULL bitmap research

## Sources

- MySQL source documentation: `COM_STMT_EXECUTE`, https://dev.mysql.com/doc/dev/mysql-server/latest/page_protocol_com_stmt_execute.html

## Notes

- `COM_STMT_EXECUTE` parameter payload begins with a NULL bitmap when the prepared statement has parameters.
- The NULL bitmap length is `(parameter_count + 7) / 8`.
- The execute parameter NULL bitmap uses bit offset `0`.
- Bits map to zero-based parameter indexes in little-endian bit order inside each byte.
- Bytes after the NULL bitmap include `new_params_bind_flag`, optional parameter type metadata, and parameter values.

## Task Scope Decision

This task decodes only the NULL bitmap and stores zero-based NULL parameter indexes in MySQL-local execute envelope state. It does not decode the new-parameter-bound flag, parameter types, parameter values, expanded SQL, or redaction behavior.
