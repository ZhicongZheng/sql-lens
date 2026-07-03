# Rust Workspace Skeleton

## Goal

Create the first implementation slice for SQL Lens: a minimal Rust Cargo workspace skeleton that establishes crate boundaries without adding business logic.

## User Value

The workspace skeleton gives future contributors and AI coding agents a concrete place to implement core, proxy, protocol, storage, API, plugin, and app code. It turns the documentation architecture into a compilable project foundation.

## Background

The previous completed task created the initial SQL Lens documentation set and Git repository.

Repository evidence establishes the intended first implementation step:

- `ROADMAP.md` defines v0.1 as Proxy Foundation.
- `MILESTONE.md` starts with Milestone 1: Repository And Workspace Foundation.
- `ISSUES.md` starts with Issue 001: Create Rust workspace skeleton.
- `README.md`, `ARCHITECTURE.md`, `AGENTS.md`, and `.trellis/spec/backend/directory-structure.md` consistently define the same Rust crate split.

Cargo official documentation supports a virtual workspace manifest with `members`, explicit `resolver`, `workspace.package`, and `workspace.dependencies`.

Local toolchain:

- `rustc 1.91.1`
- `cargo 1.91.1`

## Scope

Create only the Cargo workspace skeleton:

- Root `Cargo.toml`.
- Root `.gitignore` for Cargo build output.
- `crates/` directory.
- Eight documented workspace members:
  - `sql-lens-core`
  - `sql-lens-proxy`
  - `sql-lens-protocol`
  - `sql-lens-protocol-mysql`
  - `sql-lens-storage`
  - `sql-lens-api`
  - `sql-lens-plugin`
  - `sql-lens-app`
- Minimal `Cargo.toml` for each crate.
- Minimal `src/lib.rs` for library crates.
- Minimal `src/main.rs` for the app crate.
- `sql-lens-app` package exposes a binary named `sql-lens`.

## Technical Decisions

- Use a virtual root workspace manifest.
- Use `resolver = "3"`.
- Use Rust `edition = "2024"`.
- Use `rust-version = "1.85"`.
- Use `sql-lens-app` as the application package name.
- Use `sql-lens` as the binary name.
- Do not optimize for old Rust versions.
- Do not add CI/dev tooling in this task.
- Do not add proxy, protocol, storage, API, plugin, or frontend business behavior.

## Requirements

- Workspace builds with all members.
- Crate names match existing documentation exactly.
- Crate responsibilities are represented only by minimal module comments or placeholder items, not by business logic.
- Root workspace metadata should avoid unnecessary duplication in member manifests.
- Validation should use Cargo commands only.
- The change should remain small enough for one PR.

## Out Of Scope

- TCP proxy implementation.
- Protocol adapter traits.
- MySQL parser implementation.
- Capture event models.
- Storage implementation.
- REST or WebSocket implementation.
- Frontend app skeleton.
- CI/GitHub Actions.
- rustfmt/clippy policy files.
- Release packaging.
- Publishing crates.

## Acceptance Criteria

- [ ] Root `Cargo.toml` exists and is a virtual workspace manifest.
- [ ] Root `.gitignore` ignores Cargo `target/` build output.
- [ ] Workspace uses `resolver = "3"`.
- [ ] Workspace packages use Rust edition 2024.
- [ ] Workspace packages declare MSRV `1.85`.
- [ ] All eight documented crates exist under `crates/`.
- [ ] Seven library crates contain minimal `src/lib.rs`.
- [ ] `sql-lens-app` contains minimal `src/main.rs`.
- [ ] `sql-lens-app` exposes a binary named `sql-lens`.
- [ ] `cargo fmt --check` passes.
- [ ] `cargo check --workspace` passes.
- [ ] `cargo test --workspace` passes.
- [ ] No business logic is introduced.

## Open Questions

None blocking.
