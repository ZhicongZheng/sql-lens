# MySQL COM_STMT_PREPARE response research

## Sources

- MySQL source documentation: `COM_STMT_PREPARE`, https://dev.mysql.com/doc/dev/mysql-server/latest/page_protocol_com_stmt_prepare.html

## Notes

- `COM_STMT_PREPARE` starts server-side prepared statement creation.
- The first successful backend response is `COM_STMT_PREPARE_OK`.
- The OK response starts with status byte `0x00`.
- The successful response includes a server-assigned `statement_id`.
- The successful response includes `num_columns` and `num_params` counts.
- A reserved filler byte follows the counts.
- Protocol 4.1 clients include warning count in the first OK response packet.
- Optional result-set metadata capability may add a metadata-following flag.
- Failed prepare responses use the regular MySQL ERR packet shape.
- Parameter definition packets and column definition packets follow only when counts are non-zero; parsing them belongs to later tasks.

## Task Scope Decision

This task should parse only the first prepare response packet and ERR packet. It should store a MySQL-local prepare outcome that later statement-map work can consume, but it should not build the statement ID map itself.
