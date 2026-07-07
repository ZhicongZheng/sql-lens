# Parse MySQL packet header plan

## Checklist

- [x] Read backend specs before implementation.
- [x] Add `packet.rs` to `sql-lens-protocol-mysql`.
- [x] Define packet header, packet view, parser error, and parser function.
- [x] Implement parser with no allocation on successful path.
- [x] Re-export packet types from crate root.
- [x] Add unit tests for normal packet, empty payload, short header, incomplete payload, and trailing bytes.
- [x] Update backend spec with MySQL packet header parser contract.
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

- Keep stream buffering/reassembly out of this task.
- Keep payload parsing out of this task.
- Do not emit SQL events from packet parser tests.
- Do not add third-party parsing dependencies.
