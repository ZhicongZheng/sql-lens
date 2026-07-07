# Implementation Plan

## Checklist

- [ ] Add a MySQL-local expanded SQL renderer module or section in
      `execute.rs`.
- [ ] Add `MysqlExpandedSqlRenderError` with structured mismatch variants.
- [ ] Implement placeholder scanning that skips quoted strings, quoted
      identifiers, and comments.
- [ ] Implement `SqlParameterValue` literal rendering.
- [ ] Add `expanded_sql: Option<String>` to
      `MysqlStatementExecuteEnvelope`.
- [ ] Wire adapter execute-envelope construction to render when decoded
      parameters are complete.
- [ ] Add parser/renderer tests for:
      - strings with quotes
      - `NULL`
      - numeric values
      - boolean values
      - date/time/timestamp values
      - JSON values
      - binary summaries
      - skipped placeholders inside quotes/comments
      - too few parameters
      - too many parameters
- [ ] Add adapter test proving forwarded observation remains byte-count only and
      envelope stores expanded SQL.
- [ ] Update backend spec with the expanded SQL rendering contract.

## Validation

- `rtk cargo fmt --check`
- `rtk cargo test -p sql-lens-protocol-mysql`
- `rtk cargo test --workspace`
- `rtk cargo clippy --workspace --all-targets -- -D warnings`
- Debug-output scan for `tracing::`, `println!`, `eprintln!`, and `dbg!` in the
  touched MySQL protocol files.

## Rollback Points

- If the scanner becomes too broad, reduce support to quoted strings plus
  comments and keep unsupported SQL contexts as skipped future work.
- If adapter wiring risks exposing unredacted expanded SQL outside MySQL-local
  state, keep this task renderer-only and defer envelope storage.
