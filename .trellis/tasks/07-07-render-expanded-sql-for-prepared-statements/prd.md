# Render expanded SQL for prepared statements

## Goal

Implement Issue 055 by rendering a readable expanded SQL string for
MySQL-compatible prepared statement executions.

The renderer replaces statement-template placeholders with decoded parameter
literals for debugging display, while never modifying forwarded traffic.

## Confirmed Facts

- Issue 055 is `P0`, `Hard`, and labeled `area:core`,
  `area:protocol-mysql`, and `type:feature`.
- `SqlEvent` already has `expanded_sql: Option<String>`.
- `SqlParameterValue` already models null, numeric, boolean, string,
  date/time/timestamp, JSON, binary summary, and unsupported values.
- MySQL prepared execute envelopes now store decoded MySQL-local parameters in
  `MysqlStatementExecuteEnvelope.parameters`.
- Prepared statement execute events are not emitted yet, so the first
  implementation should keep expanded SQL MySQL-local.

## Requirements

- Render expanded SQL from a prepared statement template and decoded parameters.
- Replace only `?` placeholders that are outside SQL strings, quoted
  identifiers, and comments.
- Render `NULL` values as `NULL`.
- Render numeric values without quotes.
- Render booleans as `TRUE` or `FALSE`.
- Render strings, dates, times, timestamps, JSON, binary summaries, and
  unsupported values as quoted display literals.
- Quote string-like values with single quotes and escape embedded single quotes.
- Render binary values as summaries, not raw bytes.
- Return a structured render error on placeholder/parameter count mismatch.
- Store the rendered string only on MySQL-local execute envelope state in this
  milestone.
- Never modify client-to-backend forwarded bytes.

## Acceptance Criteria

- [x] Strings are quoted and escaped.
- [x] `NULL` renders as `NULL`.
- [x] Binary values render as summaries.
- [x] Placeholder scanning skips `?` inside strings, quoted identifiers, and
      comments.
- [x] Parameter count mismatches return structured errors.
- [x] Rendering never modifies forwarded traffic.

## Out Of Scope

- Emitting `SqlEvent` for prepared statement executions.
- Storage, API, WebSocket, UI, or plugin changes.
- Redaction policy.
- SQL normalization or fingerprinting.
- Dialect-perfect executable SQL reconstruction.
- Cross-execute parameter type caching for `new_params_bind_flag = 0`.
