# Issue 094: Add GitHub Actions CI for Rust

## Goal

Add GitHub Actions CI jobs for Rust format, lint, and tests.

## Requirements

- Add a GitHub Actions workflow for Rust backend checks.
- Run on pushes to `main` and pull requests targeting `main`.
- Include separate or clearly named CI steps for:
  - `cargo fmt --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
- Use stable Rust on `ubuntu-latest`.
- Use dependency/build caching for Cargo registry/git data and `target/`.
- Keep frontend CI, Docker CI, release packaging, and markdown linting out of scope.

## Acceptance Criteria

- [x] `.github/workflows/rust-ci.yml` exists.
- [x] Workflow has a Rust format check.
- [x] Workflow has a Rust clippy check.
- [x] Workflow has a Rust workspace test check.
- [x] Workflow runs on `push` and `pull_request` for `main`.
- [x] Workflow caches Cargo dependencies/build outputs.
- [x] Local validation passes:
  - [x] `rtk cargo fmt --check`
  - [x] `rtk cargo test --workspace`
  - [x] `rtk cargo clippy --workspace --all-targets -- -D warnings`

## Notes

- Keep this task CI-only. Do not modify Rust source code unless local validation exposes a pre-existing check failure that must be fixed separately.
