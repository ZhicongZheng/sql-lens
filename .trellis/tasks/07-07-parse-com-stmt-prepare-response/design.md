# Parse COM_STMT_PREPARE response design

## Boundary

Implement in `crates/sql-lens-protocol-mysql`.

Do not change core models, capture events, storage, API, WebSocket, frontend, plugin, proxy, or app runtime code. Statement ID mapping remains a later MySQL-local state task.

## Protocol Shape

Official MySQL protocol documentation describes a successful prepare response as `COM_STMT_PREPARE_OK`.

The first response packet contains:

- status byte `0x00`.
- little-endian `statement_id` as 4 bytes.
- little-endian `num_columns` as 2 bytes.
- little-endian `num_params` as 2 bytes.
- reserved filler byte.
- optional warning count for protocol 4.1 clients.
- optional metadata-following flag when optional result-set metadata capability applies.

This task needs only the first response packet fields required by Issue 048:

```rust
pub struct MysqlComStmtPrepareOk {
    pub statement_id: u32,
    pub num_columns: u16,
    pub num_params: u16,
    pub warning_count: Option<u16>,
}
```

Failure responses use the existing MySQL ERR packet summary:

```rust
pub enum MysqlComStmtPrepareResponse {
    Ok(MysqlComStmtPrepareOk),
    Error(MysqlErrPacketSummary),
}
```

Recommended parser API:

```rust
pub fn parse_com_stmt_prepare_response(
    payload: &[u8],
) -> Result<Option<MysqlComStmtPrepareResponse>, MysqlComStmtPrepareResponseParseError>;
```

Return `Ok(None)` for packet types that are neither prepare OK nor ERR. Return structured parse errors for incomplete OK payloads. Reuse existing ERR parser behavior for error packets.

## State Shape

Keep a MySQL-local outcome record. This bridges Issue 048 and Issue 049 without prematurely building the connection statement map.

```rust
pub struct MysqlStatementPrepareOutcome {
    pub command: MysqlClientCommand,
    pub response_sequence_id: u8,
    pub response: MysqlStatementPrepareResponseState,
}

pub enum MysqlStatementPrepareResponseState {
    Prepared {
        statement_id: u32,
        num_columns: u16,
        num_params: u16,
        warning_count: Option<u16>,
    },
    Failed {
        error: MysqlErrPacketSummary,
    },
}
```

Expose:

```rust
impl MysqlConnectionState {
    pub fn last_statement_prepare_outcome(&self) -> Option<&MysqlStatementPrepareOutcome>;
}
```

## Adapter Behavior

- If phase is not `Authenticated`, backend bytes are counted but prepare response parsing does not run.
- If no pending prepare exists, backend prepare response packets are non-fatal and emit zero events.
- If a pending prepare exists and the backend payload is a valid prepare OK:
  - take `pending_statement_prepare`;
  - store `last_statement_prepare_outcome` with the original command, response sequence ID, statement ID, column count, parameter count, and optional warning count;
  - emit zero events.
- If a pending prepare exists and the backend payload is a valid ERR packet:
  - take `pending_statement_prepare`;
  - store a failed prepare outcome with the parsed ERR summary;
  - emit zero events.
- If a pending prepare exists and parsing fails because the response is incomplete or malformed:
  - keep `pending_statement_prepare`;
  - do not update last outcome;
  - emit zero events.

This preserves non-fatal observation while avoiding stale pending prepares after terminal OK/ERR responses.

## Compatibility

- `COM_QUERY` response finalization must remain unchanged.
- Existing OK and ERR summary parsers should not be broadened for prepare unless a shared helper naturally fits.
- The new parser should live in a MySQL-specific module, likely `prepare.rs`, and be re-exported from `lib.rs`.
- No new dependencies.

## Trade-Offs

- Storing only the last prepare outcome is intentionally temporary. It satisfies Issue 048 and gives Issue 049 a clean input without introducing map lifecycle rules early.
- Warning count is useful and cheap because it is in the first OK packet. Parameter and column definition packets are deliberately excluded because they require result-set packet sequencing.

## Rollback

If the outcome state proves too broad during implementation, keep only parser-level support and adapter tests proving pending state is consumed. Do not build the statement map as a workaround.
