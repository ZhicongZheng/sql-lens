# Issue 114 Design

## Problem

The MySQL adapter starts a pending `COM_QUERY` when it observes client command
bytes, then emits a `SqlEvent` when backend bytes contain a terminal OK or ERR
packet. Result-set queries do not use that one-packet terminal shape. A normal
`SELECT` response is:

1. column count packet
2. column definition packets
3. EOF or OK packet after columns
4. zero or more row packets
5. EOF or OK packet after rows

Current code treats that first column count packet as unsupported and keeps the
pending query forever, so no event reaches storage/API/UI.

## Boundaries

- Implement this in `sql-lens-protocol-mysql`.
- Keep `sql-lens-app` runtime fan-out unchanged except for test coverage.
- Do not change shared `SqlEvent` schema.
- Do not store result row values.
- Preserve existing OK and ERR packet finalization.

## Design

Add a small MySQL-local result-set tracker to `MysqlConnectionState`.

State shape:

```rust
enum MysqlQueryResponseState {
    Columns { remaining_columns: u64 },
    Rows { returned_rows: u64 },
}
```

Flow:

1. When a backend packet arrives with a pending `COM_QUERY`, first keep existing
   OK/ERR handling.
2. If no result-set tracker exists, parse the packet payload as a length-encoded
   integer column count. If the count is greater than zero, enter
   `Columns { remaining_columns: count }`.
3. While in `Columns`, count down one column definition packet per backend
   packet. When all columns have been seen, wait for the column terminator
   EOF/OK packet and then enter `Rows { returned_rows: 0 }`.
4. While in `Rows`, each non-terminal packet increments `returned_rows`.
5. When a row terminator EOF/OK packet arrives, emit the pending query event
   with `status = ok` and `ResultSummary.returned_rows = Some(returned_rows)`.
6. Leave malformed or incomplete packets non-fatal and keep state unchanged.

This is intentionally packet-sequence oriented. It does not decode column or row
contents.

## SQL Extraction Fix

`COM_QUERY` SQL must come from the command payload after the command byte. If
tests reveal packet header bytes or command bytes are retained in `original_sql`,
fix the parser in the MySQL command module and add regression tests.

## Compatibility

- Existing OK/ERR `COM_QUERY` behavior stays first in the decision tree.
- Prepared statement execute OK/ERR behavior stays unchanged.
- If a result-set packet sequence is split across TCP reads, current adapter
  assumptions may still defer support; this task covers complete packets as the
  current parser model does.

## Validation

- Protocol unit tests for result-set query finalization and clean SQL.
- Existing MySQL protocol tests remain green.
- Docker-only MySQL smoke adds a proxied `SELECT` API assertion.
