# Rust Workspace Skeleton Design

## Objective

Create a minimal Cargo workspace that matches the documented SQL Lens backend architecture while keeping the first implementation slice intentionally small.

## Workspace Shape

Use a virtual workspace manifest at repository root:

```toml
[workspace]
resolver = "3"
members = [
  "crates/sql-lens-core",
  "crates/sql-lens-proxy",
  "crates/sql-lens-protocol",
  "crates/sql-lens-protocol-mysql",
  "crates/sql-lens-storage",
  "crates/sql-lens-api",
  "crates/sql-lens-plugin",
  "crates/sql-lens-app",
]
```

Use workspace package inheritance for common metadata:

- version
- edition
- rust-version
- license
- repository

## Crates

### `sql-lens-core`

Library crate for protocol-neutral models. Skeleton only.

### `sql-lens-proxy`

Library crate for future TCP proxy runtime. Skeleton only.

### `sql-lens-protocol`

Library crate for future protocol adapter traits. Skeleton only.

### `sql-lens-protocol-mysql`

Library crate for future MySQL-compatible adapter. Skeleton only.

### `sql-lens-storage`

Library crate for future ring buffer, SQLite, and DuckDB storage implementations. Skeleton only.

### `sql-lens-api`

Library crate for future REST and WebSocket APIs. Skeleton only.

### `sql-lens-plugin`

Library crate for future plugin and exporter contracts. Skeleton only.

### `sql-lens-app`

Binary package for application composition.

Package name: `sql-lens-app`.

Binary name: `sql-lens`.

The initial `main` should do no real work. It may print a minimal placeholder or remain empty if that keeps checks simple.

## Dependency Strategy

For this skeleton task, avoid inter-crate dependencies unless Cargo requires them.

Rationale:

- The crate graph should not imply APIs that do not exist yet.
- Dependency edges should be introduced when the first real public contract is added.
- This preserves YAGNI and prevents false architecture.

## Compatibility

Rust settings:

- edition 2024.
- MSRV 1.85.
- resolver 3.

The installed local toolchain is newer and supports these settings.

## Validation

Run:

```bash
rtk cargo fmt --check
rtk cargo check --workspace
rtk cargo test --workspace
```

No JetBrains compile/test requirement applies yet because there is no existing project model, but JetBrains can be used later once the Rust project is imported.

## Risks

- Adding dependencies too early creates artificial coupling.
- Adding placeholder APIs too early can become accidental public contract.
- Naming drift from docs would confuse later contributors.

## Rollback

Rollback is simple: remove root `Cargo.toml` and the `crates/` directory created by this task.

