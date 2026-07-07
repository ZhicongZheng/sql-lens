# Decode common MySQL numeric parameters

## Goal

Plan Issue 052: decode common numeric MySQL prepared statement parameters from `COM_STMT_EXECUTE` packets.

## Background

- Issue 050 parses the `COM_STMT_EXECUTE` envelope.
- Issue 051 decodes the execute NULL bitmap and stores zero-based NULL parameter indexes.
- MySQL numeric parameter values require parameter type metadata before values can be decoded safely.
- MySQL can send parameter type metadata only when `new_params_bind_flag = 1`; later executions may reuse previous metadata with `new_params_bind_flag = 0`.
- `sql-lens-core` already has protocol-neutral parameter value variants:
  - `SqlParameterValue::Null`
  - `SqlParameterValue::Integer(i64)`
  - `SqlParameterValue::Unsigned(u64)`
  - `SqlParameterValue::Float(f64)`

## Requirements

- Decode signed integer parameter classes from execute packets.
- Decode unsigned integer parameter classes from execute packets.
- Decode `FLOAT` and `DOUBLE` values from execute packets.
- Preserve zero-based parameter indexes.
- Preserve NULL parameters from the existing NULL bitmap decoder.
- Return structured parse errors for truncated parameter type metadata or values.
- Keep adapter observation non-fatal for malformed numeric parameter payloads.
- Do not decode string, binary, date/time, decimal, JSON, or complex values in this task.
- Support only execute packets with `new_params_bind_flag = 1`.
- Treat `new_params_bind_flag = 0` as non-fatal unsupported until a later per-statement type-cache task.
- Do not render expanded SQL in this task.
- Do not emit `SqlEvent` in this task unless a later design decision explicitly changes scope.
- Do not add new dependencies.

## Acceptance Criteria

- [ ] Signed integer representative values are decoded.
- [ ] Unsigned integer representative values are decoded.
- [ ] `FLOAT` representative values are decoded.
- [ ] `DOUBLE` representative values are decoded.
- [ ] NULL numeric parameters are represented as NULL and do not consume value bytes.
- [ ] Truncated parameter type metadata returns a structured parse error.
- [ ] Truncated numeric value bytes return a structured parse error.
- [ ] Adapter behavior remains non-fatal for malformed numeric payloads.
- [ ] Existing execute envelope and NULL bitmap tests remain green.
- [ ] `cargo fmt --check` passes.
- [ ] `cargo test -p sql-lens-protocol-mysql` passes.
- [ ] `cargo test --workspace` passes.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes.

## Out of Scope

- String and binary parameter decoding.
- Date and time parameter decoding.
- Decimal parameter decoding.
- Expanded SQL rendering.
- Redaction and persistence behavior.
- Cross-execute parameter type cache unless explicitly approved.
- Storage, API, WebSocket, UI, proxy, app runtime, and plugin changes.

## Scope Decision

- Issue 052 supports only execute packets with `new_params_bind_flag = 1`, and treats `new_params_bind_flag = 0` as non-fatal unsupported until a later per-statement type-cache task. This keeps the task independently reviewable and avoids hidden statefulness before parameter decoding is proven.
