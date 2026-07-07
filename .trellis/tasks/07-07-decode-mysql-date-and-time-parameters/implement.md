# Implementation Plan

## Checklist

- [ ] Add temporal type-code constants to `execute.rs`.
- [ ] Add temporal parse helpers for date/datetime/timestamp and time payloads.
- [ ] Extend `decode_parameter_value` with temporal branches.
- [ ] Add parser tests for:
      - `DATE`
      - zero date
      - `TIME`
      - negative time
      - microsecond time
      - `DATETIME`
      - `TIMESTAMP`
      - microsecond datetime/timestamp
      - unsupported temporal length
      - truncated temporal payload
- [ ] Add adapter coverage proving known statement IDs store decoded temporal
      parameters.
- [ ] Update backend spec with the temporal parameter contract.

## Validation

- `rtk cargo fmt --check`
- `rtk cargo test -p sql-lens-protocol-mysql`
- `rtk cargo test --workspace`
- `rtk cargo clippy --workspace --all-targets -- -D warnings`
- Debug-output scan for `tracing::`, `println!`, `eprintln!`, and `dbg!` in the
  touched MySQL protocol files.

## Rollback Points

- If temporal formatting becomes unclear, keep parse helpers private and avoid
  changing public contracts until the representation is settled.
- If alternate `TIME2`/`DATETIME2`/`TIMESTAMP2` types require a different value
  shape, defer those variants instead of weakening the first temporal contract.
