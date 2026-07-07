# MySQL COM_STMT_PREPARE Research

## Sources

- MySQL source documentation: `COM_STMT_PREPARE`, https://dev.mysql.com/doc/dev/mysql-server/latest/page_protocol_com_stmt_prepare.html

## Notes

- `COM_STMT_PREPARE` starts a server-side prepared statement lifecycle.
- The command byte is `0x16`.
- The rest of the client command payload is the query string, read to EOF.
- The server response includes statement metadata such as statement ID, column count, parameter count, and warning count, but parsing that response belongs to a later task.

## Task Scope Decision

This task should parse the client command and store a pending statement-prepare template. It should not parse the backend prepare OK response or create a completed prepared-statement record.
