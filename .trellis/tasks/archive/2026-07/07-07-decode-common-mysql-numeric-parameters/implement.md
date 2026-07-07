# Decode common MySQL numeric parameters plan

## Checklist

- [x] Resolve `new_params_bind_flag = 0` scope with the user.
- [x] Read backend specs before implementation.
- [x] Read numeric parameter research notes.
- [x] Add numeric parameter metadata parser.
- [x] Decode signed integer values.
- [x] Decode unsigned integer values.
- [x] Decode `FLOAT`.
- [x] Decode `DOUBLE`.
- [x] Preserve NULL parameters without consuming value bytes.
- [x] Add structured errors for truncated metadata.
- [x] Add structured errors for truncated values.
- [x] Decide MySQL-local vs core parameter state shape.
- [x] Integrate decoded numeric parameters into execute envelope state if in scope.
- [x] Keep unsupported or malformed payloads non-fatal at adapter level.
- [x] Update backend spec with numeric parameter contract.
- [x] Run `rtk cargo fmt --check`.
- [x] Run `rtk cargo test -p sql-lens-protocol-mysql`.
- [x] Run `rtk cargo test --workspace`.
- [x] Run `rtk cargo clippy --workspace --all-targets -- -D warnings`.

## Validation Commands

```bash
rtk cargo fmt --check
rtk cargo test -p sql-lens-protocol-mysql
rtk cargo test --workspace
rtk cargo clippy --workspace --all-targets -- -D warnings
```

## Risk Notes

- Do not build cross-execute type cache unless explicitly approved.
- Do not decode string, binary, date/time, decimal, or JSON values.
- Do not render expanded SQL.
- Do not emit SQL events.
- Do not add dependencies.
