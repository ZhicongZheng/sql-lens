# Parse COM_STMT_EXECUTE envelope

## Goal

Implement Issue 050: parse the MySQL `COM_STMT_EXECUTE` client command envelope and connect it to connection-local prepared statement metadata when available.

## Background

- Issue 047 parses `COM_STMT_PREPARE`.
- Issue 048 parses prepare OK/ERR responses.
- Issue 049 stores prepared statement mappings per `MysqlConnectionState`.
- `COM_STMT_EXECUTE` is the entry point for prepared statement execution.
- Later tasks decode NULL bitmap, parameter types, parameter values, expanded SQL, and close/reset cleanup.

## Requirements

- Parse client command payloads with command byte `0x17` as `COM_STMT_EXECUTE`.
- Extract server statement ID.
- Extract execute flags.
- Extract iteration count.
- Detect whether the packet has a parameter metadata marker when the referenced prepared statement has parameters.
- Look up statement metadata in the current connection-local prepared statement map.
- Report unknown statement IDs gracefully without failing adapter observation.
- Keep `COM_STMT_EXECUTE` before authentication non-fatal and ignored.
- Keep malformed `COM_STMT_EXECUTE` packets non-fatal at adapter level.
- Do not decode NULL bitmap in this task.
- Do not decode parameter types or values in this task.
- Do not render expanded SQL in this task.
- Do not emit `SqlEvent` for execute parsing in this task.
- Do not add new dependencies.
- Represent unknown statement IDs as a successfully parsed execute envelope with `statement: None`.

## Acceptance Criteria

- [ ] Parser extracts statement ID from a valid `COM_STMT_EXECUTE` fixture.
- [ ] Parser extracts flags from a valid `COM_STMT_EXECUTE` fixture.
- [ ] Parser extracts iteration count from a valid `COM_STMT_EXECUTE` fixture.
- [ ] Parser detects parameter metadata marker presence when bytes are available after the envelope.
- [ ] Parser rejects incomplete execute envelope payloads with structured errors.
- [ ] Adapter stores a MySQL-local last execute envelope after authentication.
- [ ] Adapter links execute envelope to prepared statement metadata when statement ID is known.
- [ ] Adapter records unknown statement IDs gracefully without failing observation.
- [ ] `COM_STMT_EXECUTE` before authentication does not update state.
- [ ] Existing `COM_QUERY` and prepare behavior remains unchanged.
- [ ] `cargo fmt --check` passes.
- [ ] `cargo test -p sql-lens-protocol-mysql` passes.
- [ ] `cargo test --workspace` passes.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes.

## Out of Scope

- NULL bitmap decoding.
- Parameter type decoding.
- Parameter value decoding.
- Expanded SQL rendering.
- SQL event emission, storage, API, WebSocket, UI, and plugin changes.
- Statement close/reset cleanup.
- Protocol-neutral core model changes.

## Scope Decision

- Unknown statement IDs are represented as a successful parsed execute envelope with `statement: None`, rather than as an adapter error. This keeps proxy observation non-fatal and gives later tasks a clear branch for unsupported or unknown executions.
