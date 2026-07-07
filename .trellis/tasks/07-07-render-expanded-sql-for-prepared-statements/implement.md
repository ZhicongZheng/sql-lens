# Implementation Plan

## Checklist

- [x] Add a MySQL-local expanded SQL renderer module or section in
      `execute.rs`.
- [x] Add `MysqlExpandedSqlRenderError` with structured mismatch variants.
- [x] Implement placeholder scanning that skips quoted strings, quoted
      identifiers, and comments.
- [x] Implement `SqlParameterValue` literal rendering.
- [x] Add `expanded_sql: Option<String>` to
      `MysqlStatementExecuteEnvelope`.
- [x] Wire adapter execute-envelope construction to render when decoded
      parameters are complete.
- [x] Add parser/renderer tests for:
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
- [x] Add adapter test proving forwarded observation remains byte-count only and
      envelope stores expanded SQL.
- [x] Update backend spec with the expanded SQL rendering contract.

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
