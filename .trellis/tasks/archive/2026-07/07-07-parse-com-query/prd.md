# Parse COM_QUERY

## Goal

Implement Issue 043: parse MySQL `COM_QUERY` command payloads, extract SQL text safely, and record the command type for later timing and event capture tasks.

## Background

- Issue 038 added MySQL packet envelope parsing.
- Issue 040 observes the server initial handshake.
- Issue 041 observes the client handshake response.
- Issue 042 detects authentication success or failure and moves state to `Authenticated` or `AuthenticationFailed`.
- Milestone 8 starts text-query capture, but timing, backend response finalization, OK/ERR summaries, and SQL event emission are later issues.

## Requirements

- Observe client command packets only after the MySQL connection phase is `Authenticated`.
- Detect MySQL `COM_QUERY` command payloads by command byte `0x03`.
- Extract SQL text from the remaining payload bytes.
- Validate SQL text as UTF-8 for the first parser layer.
- Store safe parsed command metadata in MySQL-specific connection state:
  - command type,
  - SQL text,
  - packet sequence ID when available.
- Treat unsupported commands as non-fatal and non-transitioning.
- Treat malformed or invalid UTF-8 command payloads as non-fatal in adapter observation.
- Emit no `SqlEvent` records from command parsing in this task.

## Acceptance Criteria

- [x] `COM_QUERY` payloads parse into command type plus SQL text.
- [x] Empty `COM_QUERY` SQL text is handled deterministically.
- [x] Invalid UTF-8 returns a structured parser error.
- [x] Unsupported command bytes return a non-fatal unsupported result.
- [x] Client command packets before `Authenticated` do not update command state.
- [x] Valid `COM_QUERY` after `Authenticated` stores safe command metadata in `MysqlConnectionState`.
- [x] Malformed or invalid `COM_QUERY` after `Authenticated` does not fail adapter observation.
- [x] The adapter emits no `SqlEvent` records for command parsing.
- [x] `cargo fmt --check` passes.
- [x] `cargo test -p sql-lens-protocol-mysql` passes.
- [x] `cargo test --workspace` passes.
- [x] `cargo clippy --workspace --all-targets -- -D warnings` passes.

## Out of Scope

- Measuring query duration.
- Waiting for backend OK/ERR/result-set responses.
- Emitting `SqlEvent`.
- SQL normalization, fingerprinting, or parameter expansion.
- Prepared statements.
- Multi-packet command reassembly.
- Character-set-specific decoding beyond UTF-8.
- Query redaction.
