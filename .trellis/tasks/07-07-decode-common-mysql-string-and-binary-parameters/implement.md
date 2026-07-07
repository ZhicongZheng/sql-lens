# Implementation Plan

## Checklist

- [x] Extend `execute.rs` with length-encoded byte-string parsing.
- [x] Add text and binary type-code constants.
- [x] Generalize the value decoder so numeric, text, binary, and NULL values
      share one decode path.
- [x] Keep `decode_numeric_parameters` as a wrapper or update all call sites to
      the new general decoder.
- [x] Rename execute envelope decoded parameter state from
      `numeric_parameters` to `parameters`.
- [x] Add parser unit tests for:
      - valid text
      - invalid UTF-8 text
      - binary summary with short and truncated-prefix display
      - mixed numeric/text/binary/NULL parameters
      - truncated length prefix
      - truncated string or binary value bytes
- [x] Add adapter coverage proving known statement IDs store decoded text and
      binary summary parameters.
- [x] Update backend spec with the new string/binary parameter contract.

## Validation

- `rtk cargo fmt --check`
- `rtk cargo test -p sql-lens-protocol-mysql`
- `rtk cargo test --workspace`
- `rtk cargo clippy --workspace --all-targets -- -D warnings`
- Debug-output scan for `tracing::`, `println!`, `eprintln!`, and `dbg!` in the
  touched MySQL protocol files.

## Rollback Points

- If length-encoded parsing complicates the decoder, keep numeric decoding as
  the stable fallback and isolate string/binary support behind a separate helper.
- If renaming `numeric_parameters` creates broad fallout, keep a compatibility
  alias only inside MySQL-local state and avoid touching core contracts.
