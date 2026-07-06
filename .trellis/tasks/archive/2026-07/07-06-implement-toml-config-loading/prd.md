# Implement TOML Config Loading

## Goal

Implement Issue 008: load SQL Lens startup configuration from a TOML file into the existing `sql-lens-config` model.

## User Value

Developers need a real `sql-lens.toml` loading path before CLI startup, validation, logging initialization, runtime startup, and future hot reload can be implemented safely.

## Background

- Issue 007 is complete: `sql-lens-config` owns typed configuration structs, enums, defaults, serde derives, and lightweight tests.
- Issue 008 requires loading configuration from a TOML file.
- `CONFIG.md` declares TOML as the recommended format and documents the example file shape.
- `sql-lens-app` will later own CLI and runtime startup. This task should keep file parsing inside `sql-lens-config` and avoid starting services.
- Context7 confirmed `toml::from_str` deserializes TOML into serde-backed Rust types and requires the `parse` and `serde` crate features.

## Requirements

- Add TOML loading support to `sql-lens-config`.
- Provide a public API to load `SqlLensConfig` from a filesystem path.
- Provide a public API to parse `SqlLensConfig` from a TOML string for tests and future callers that already have config content in memory.
- Missing sections and fields should use existing `Default` values so users can write minimal config files.
- Unknown sections and fields should be rejected during deserialization so typos do not silently fall back to defaults.
- Return a structured error type that distinguishes file read failures from TOML parse/deserialization failures.
- The structured error type should implement `Debug`, `Display`, and `std::error::Error`.
- Tests must cover valid config loading from a path.
- Tests must cover invalid TOML returning a structured parse error.
- Tests must cover missing-file read errors.
- Tests must cover default fallback behavior for partial TOML.

## Dependency Policy

- Allow adding `toml` to `sql-lens-config`.
- Prefer minimal TOML features: `parse` and `serde`.
- Do not add `tempfile`; use standard library helpers for test files.
- Do not add validation crates.
- Do not depend on `sql-lens-core`.
- Do not add runtime, CLI, watcher, HTTP, storage, or logging dependencies.

## Out Of Scope

- Environment variable overrides.
- Config validation rules such as required listen address or backend address.
- CLI `--config`.
- Runtime startup.
- Hot reload.
- File watching.
- SQLite runtime settings.
- Logging initialization.
- Proxy, API, or frontend behavior.

## Acceptance Criteria

- [ ] `sql-lens-config` can load a valid TOML file from a path.
- [ ] `sql-lens-config` can parse TOML from a string.
- [ ] Missing TOML sections and fields are filled from existing defaults.
- [ ] Unknown TOML sections or fields fail deserialization.
- [ ] Invalid TOML returns a structured parse error.
- [ ] Missing files return a structured read error.
- [ ] The error type implements `Display` and `std::error::Error`.
- [ ] Tests cover valid, invalid, missing-file, and partial-config cases.
- [ ] `cargo fmt --check` passes.
- [ ] `cargo check --workspace` passes.
- [ ] `cargo test --workspace` passes.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes.
- [ ] No env override, validation, CLI, hot reload, file watcher, SQLite settings, proxy, API, or frontend logic is introduced.

## Open Questions

None blocking.
