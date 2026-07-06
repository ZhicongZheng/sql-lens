# Add CLI Entry Point

## Goal

Implement Issue 011: add the initial `sql-lens` binary entry point with a config path argument, version output, and clear startup error display.

## User Value

Developers should be able to run a real `sql-lens` command that accepts a config file, loads and validates it, and reports clear errors before any runtime services exist.

## Background

- `sql-lens-app` is the workspace package that owns CLI and service composition.
- The binary name is already `sql-lens`, but `src/main.rs` is currently empty.
- `sql-lens-config` already provides TOML loading and semantic validation:
  - `SqlLensConfig::from_path`
  - `SqlLensConfig::validate`
  - `ConfigLoadError`
  - `ConfigValidationError`
- `CONFIG.md` documents `sql-lens.toml` as the default config path.
- `README.md` target quick start uses `sql-lens --config sql-lens.toml`.
- Context7 confirmed `clap` derive supports `#[derive(Parser)]`, `#[command(version, ...)]`, and long options such as `--config`.

## Requirements

- Add a real CLI implementation to `sql-lens-app`.
- Add `--config <FILE>` support.
- Default `--config` to `sql-lens.toml`.
- Support automatic `--version` output.
- Load configuration from the selected path.
- Validate loaded configuration before reporting startup success.
- Print startup errors to stderr with enough context for developers to fix the problem.
- Return a non-zero exit code on config load or validation failure.
- Return zero for successful config load and validation.
- Keep this first CLI entry point synchronous and runtime-free.
- Add tests for version output, config path handling, and startup errors.

## Dependency Policy

- Allow adding `clap` to `sql-lens-app` with the `derive` feature.
- Add a path dependency from `sql-lens-app` to `sql-lens-config`.
- Do not add async runtime dependencies.
- Do not add `assert_cmd`, `predicates`, or `tempfile`; use standard library integration tests.

## Out Of Scope

- Starting proxy, API, web UI, storage, logging, or runtime services.
- Environment variable overrides.
- Hot reload.
- Signal handling.
- Long-running daemon behavior.
- Structured logging initialization.
- Shell completion generation.
- Subcommands.
- Config file creation.

## Acceptance Criteria

- [x] `sql-lens --config <FILE>` is accepted.
- [x] `sql-lens --version` prints version information and exits successfully.
- [x] Config load errors are printed clearly to stderr.
- [x] Config validation errors are printed clearly to stderr.
- [x] Successful config load and validation exits with code zero.
- [x] Missing config file exits non-zero.
- [x] Invalid config exits non-zero.
- [x] Tests cover version output.
- [x] Tests cover valid config path success.
- [x] Tests cover missing config path failure.
- [x] Tests cover invalid config validation failure.
- [x] `cargo fmt --check` passes.
- [x] `cargo check --workspace` passes.
- [x] `cargo test --workspace` passes.
- [x] `cargo clippy --workspace --all-targets -- -D warnings` passes.
- [x] No runtime startup, proxy, API, logging initialization, env overrides, or hot reload logic is introduced.

## Open Questions

None blocking.
