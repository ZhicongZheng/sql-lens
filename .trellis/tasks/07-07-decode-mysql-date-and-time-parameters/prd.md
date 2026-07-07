# Decode MySQL date and time parameters

## Goal

Implement Issue 054 by extending the MySQL-compatible `COM_STMT_EXECUTE`
parameter decoder to handle date, time, datetime, and timestamp prepared
statement parameters.

This continues the prepared statement parameter decoding path built in Issues
050-053 and keeps decoded values MySQL-local until SQL event emission needs a
protocol-neutral conversion.

## Confirmed Facts

- Issue 054 is `P1`, `Hard`, and labeled `area:protocol-mysql` and
  `type:feature`.
- Issue 051 provides NULL bitmap decoding.
- Issue 052 provides numeric parameter decoding.
- Issue 053 provides common text and binary parameter decoding through the
  general `decode_parameters` path.
- `SqlParameterValue` already contains `Date(String)`, `Time(String)`, and
  `Timestamp(String)`, so core model changes are not required.

## Requirements

- Decode MySQL binary protocol `DATE` and `NEWDATE` values.
- Decode MySQL binary protocol `TIME` values, including negative time and
  microsecond precision when present.
- Decode MySQL binary protocol `DATETIME` and `TIMESTAMP` values, including
  microsecond precision when present.
- Represent zero-length temporal values clearly rather than panicking.
- Preserve existing numeric, text, binary, and NULL parameter behavior.
- Keep unsupported temporal variants or malformed temporal payloads non-fatal
  at adapter level.

## Acceptance Criteria

- [x] Common date values decode to `SqlParameterValue::Date`.
- [x] Common time values decode to `SqlParameterValue::Time`.
- [x] Common datetime and timestamp values decode to
      `SqlParameterValue::Timestamp`.
- [x] Zero-length and microsecond edge cases are represented clearly.
- [x] Tests cover date, time, datetime, timestamp, and malformed payloads.

## Out Of Scope

- Time zone conversion.
- Calendar validity checks beyond packet shape.
- Expanded SQL rendering.
- Redaction policy.
- Storage, API, WebSocket, UI, or plugin changes.
- Protocol-neutral core model changes.
