# Decode MySQL NULL bitmap

## Goal

Implement Issue 051: decode the MySQL `COM_STMT_EXECUTE` NULL bitmap for prepared statement executions when the connection-local prepared statement metadata is known.

## Background

- Issue 050 parses the `COM_STMT_EXECUTE` envelope and links known statement IDs to `MysqlPreparedStatement` metadata.
- MySQL prepared statement execute packets include a NULL bitmap before parameter type/value metadata when the prepared statement has parameters.
- The NULL bitmap length is derived from the prepared statement parameter count: `(num_params + 7) / 8`.
- MySQL execute parameter NULL bitmap bits are parameter-indexed with bit offset `0`.
- Later tasks decode parameter type metadata, parameter values, and expanded SQL.

## Requirements

- Decode NULL parameter positions from the execute parameter payload.
- Use the prepared statement `num_params` value to determine bitmap length.
- Use zero-based parameter indexes in Rust-facing types and tests.
- Return a structured parser error when the available bitmap bytes are shorter than the required length.
- Integrate decoded NULL parameter positions into MySQL-local execute envelope state when the statement ID is known.
- Keep unknown statement IDs non-fatal and do not attempt NULL bitmap decoding without known `num_params`.
- Keep statements with zero parameters valid with an empty NULL position list.
- Do not decode `new_params_bind_flag`.
- Do not decode parameter types.
- Do not decode parameter values.
- Do not render expanded SQL.
- Do not store raw parameter payload bytes in connection state.
- Do not emit `SqlEvent` from NULL bitmap decoding.
- Do not add new dependencies.

## Acceptance Criteria

- [ ] Parser identifies NULL parameter positions for mixed NULL and non-NULL parameters.
- [ ] Parser handles all non-NULL parameters.
- [ ] Parser handles zero parameters.
- [ ] Parser returns a structured error for truncated NULL bitmap bytes.
- [ ] Adapter stores NULL parameter positions on the last execute envelope for known statement IDs.
- [ ] Adapter keeps unknown statement IDs non-fatal with no NULL bitmap decoding.
- [ ] Adapter keeps malformed NULL bitmap packets non-fatal.
- [ ] Existing execute envelope, prepare, and query tests remain green.
- [ ] `cargo fmt --check` passes.
- [ ] `cargo test -p sql-lens-protocol-mysql` passes.
- [ ] `cargo test --workspace` passes.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes.

## Out of Scope

- Parameter type decoding.
- Parameter value decoding.
- Expanded SQL rendering.
- Redaction logic.
- Storage, API, WebSocket, UI, proxy, app runtime, and plugin changes.
- PostgreSQL or other protocol behavior.
