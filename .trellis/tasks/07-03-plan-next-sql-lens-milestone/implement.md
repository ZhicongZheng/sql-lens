# Rust Workspace Skeleton Implementation Plan

## Preconditions

- User approves this planning artifact.
- `task.py start` is run before editing implementation files.
- Work remains limited to Cargo workspace skeleton.

## Files To Create

- `Cargo.toml`
- `.gitignore`
- `crates/sql-lens-core/Cargo.toml`
- `crates/sql-lens-core/src/lib.rs`
- `crates/sql-lens-proxy/Cargo.toml`
- `crates/sql-lens-proxy/src/lib.rs`
- `crates/sql-lens-protocol/Cargo.toml`
- `crates/sql-lens-protocol/src/lib.rs`
- `crates/sql-lens-protocol-mysql/Cargo.toml`
- `crates/sql-lens-protocol-mysql/src/lib.rs`
- `crates/sql-lens-storage/Cargo.toml`
- `crates/sql-lens-storage/src/lib.rs`
- `crates/sql-lens-api/Cargo.toml`
- `crates/sql-lens-api/src/lib.rs`
- `crates/sql-lens-plugin/Cargo.toml`
- `crates/sql-lens-plugin/src/lib.rs`
- `crates/sql-lens-app/Cargo.toml`
- `crates/sql-lens-app/src/main.rs`

## Checklist

1. Create root virtual workspace `Cargo.toml`.
2. Create `crates/` member directories.
3. Create each member manifest.
4. Add minimal library source files.
5. Add minimal app source file.
6. Run formatting.
7. Run workspace check.
8. Run workspace tests.
9. Verify no business logic was added.
10. Verify crate names match documentation.

## Validation Commands

```bash
rtk cargo fmt --check
rtk cargo check --workspace
rtk cargo test --workspace
rtk rg --files Cargo.toml crates
```

## Review Gate

Do not start implementation until the user approves.

Do not add:

- CI files.
- Rustfmt config.
- Clippy config.
- Dependencies.
- Protocol models.
- Proxy runtime.
- API routes.
- Storage logic.
- Frontend files.
