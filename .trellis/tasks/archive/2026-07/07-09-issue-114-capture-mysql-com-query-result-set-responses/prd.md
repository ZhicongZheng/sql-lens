# Issue 114: Capture MySQL COM_QUERY result set responses

## Goal

Capture MySQL `COM_QUERY` statements that return result sets, such as `SELECT`,
so local SQL Lens demos show the read queries developers actually run through
the proxy.

The immediate bug report was reproduced locally: `DO 1` through
`127.0.0.1:3307` is captured and visible through SQLite/API, while
`SELECT 1` and real `SELECT * ...` queries successfully proxy to the backend
but emit no SQL event.

## Background

- `sql-lens.toml` starts the API on `127.0.0.1:5173` and the MySQL proxy on
  `127.0.0.1:3307`.
- Homebrew `mysql` client queries through `127.0.0.1:3307` successfully reached
  the configured backend.
- `DO 1` produced a stored/API-visible event because it ends with an OK packet.
- `SELECT 1` produced no event because current `COM_QUERY` finalization only
  handles OK/ERR terminal responses and not result set packet sequences.
- The captured `DO 1` event also showed leading control bytes in
  `original_sql`, so `COM_QUERY` SQL extraction must be corrected.

## Requirements

- Support `COM_QUERY` responses that return a text result set.
- Emit exactly one `SqlEventKind::Query` event after the result set reaches its
  terminal EOF/OK packet.
- Preserve existing OK-packet and ERR-packet `COM_QUERY` capture behavior.
- Populate `ResultSummary.returned_rows` for result-set queries when practical
  from packet sequencing, without storing row contents.
- Correct `COM_QUERY` SQL text extraction so captured SQL does not include the
  command byte or packet framing bytes.
- Keep malformed or incomplete result-set packet sequences non-fatal in the
  proxy hot path.
- Keep MySQL-specific packet details in protocol metadata, not top-level core
  fields.
- Do not capture result row values, result column values, or authentication
  payloads.

## Acceptance Criteria

- [ ] Protocol unit tests prove `SELECT 1`-style result set responses finalize a
      pending `COM_QUERY` event with `status = ok`.
- [ ] Protocol unit tests prove returned row count is tracked for result-set
      responses.
- [ ] Regression tests prove OK-packet and ERR-packet `COM_QUERY` finalization
      still works.
- [ ] Regression tests prove `COM_QUERY` `original_sql` is clean SQL text such
      as `DO 1` or `SELECT 1`, with no packet/control prefix.
- [ ] App integration test covers a proxied MySQL `SELECT` query through the API
      using the existing Docker-only/env-gated smoke path.
- [ ] `rtk cargo fmt --check`, targeted Rust tests, workspace tests, and clippy
      pass before commit.

## Out Of Scope

- Capturing or storing result row data.
- Full column definition parsing beyond what is needed to skip packets and
  count rows.
- Multi-packet result row reassembly beyond current adapter buffering
  assumptions.
- TLS decryption or encrypted MySQL payload inspection.
- Frontend UI changes.
