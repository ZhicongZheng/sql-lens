# Decode common MySQL string and binary parameters

## Goal

Implement Issue 053 by extending the MySQL-compatible `COM_STMT_EXECUTE`
parameter decoder to handle common string-like parameters and binary payloads
after the NULL bitmap and current-packet parameter type metadata are available.

The feature helps SQL Lens inspect prepared statement executions without
storing raw binary blobs or panicking on invalid text bytes.

## Confirmed Facts

- Issue 053 is `P0`, `Hard`, and labeled `area:protocol-mysql`,
  `area:security`, and `type:feature`.
- Issue 050 added the `COM_STMT_EXECUTE` envelope.
- Issue 051 added NULL bitmap decoding.
- Issue 052 added MySQL-local decoded numeric parameters.
- `SqlParameterValue` already contains `String(String)` and
  `BinarySummary(String)`, so this task does not need to change core models.
- The first implementation continues to support only
  `new_params_bind_flag = 1`; `new_params_bind_flag = 0` remains unsupported
  until a later per-statement parameter type cache task.

## Requirements

- Decode common text parameter type codes as length-encoded byte strings:
  `VARCHAR`, `VAR_STRING`, `STRING`, `ENUM`, and `SET`.
- Represent invalid UTF-8 text without panicking by using safe replacement
  characters.
- Decode common binary parameter type codes as summaries only:
  `TINY_BLOB`, `MEDIUM_BLOB`, `LONG_BLOB`, `BLOB`, `BIT`, and `GEOMETRY`.
- Binary summaries must include the byte length and a short hex prefix, not the
  full raw value.
- Preserve numeric parameter decoding and NULL handling from Issue 052.
- Store decoded values only in MySQL-local execute envelope state.
- Keep unknown statement IDs non-fatal and decoded parameter lists empty.
- Keep unsupported type codes non-fatal and avoid exposing partial decoded
  parameter state.
- Keep malformed parameter payloads non-fatal at adapter level by not updating
  the last client command or execute envelope.

## Acceptance Criteria

- [x] Text values are decoded safely into `SqlParameterValue::String`.
- [x] Invalid text bytes are represented without panics.
- [x] Binary values are represented as `SqlParameterValue::BinarySummary`.
- [x] Binary summaries do not contain full raw binary payloads.
- [x] Mixed numeric, text, binary, and NULL parameters decode in parameter order.
- [x] Existing numeric, NULL bitmap, prepare, query, and execute envelope tests
      continue to pass.

## Out Of Scope

- Cross-execute parameter type caching for `new_params_bind_flag = 0`.
- Date/time decoding.
- Decimal or JSON decoding.
- Expanded SQL rendering.
- Redaction policy.
- Storage, API, WebSocket, UI, or plugin changes.
- Protocol-neutral core model changes.
