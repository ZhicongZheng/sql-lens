# Add Config Validation

## Goal

Implement Issue 010: validate startup configuration after TOML loading and before services start.

## User Value

Developers should receive clear configuration errors before SQL Lens starts network listeners or runtime services. This prevents confusing startup behavior when required addresses are missing or a future protocol adapter is selected before it exists.

## Background

- Issue 007 added the `sql-lens-config` model.
- Issue 008 added TOML loading with serde defaults, unknown-field rejection, and `ConfigLoadError`.
- `CONFIG.md` lists validation rules. This task implements the first P0 subset from Issue 010.
- `Protocol` includes future variants such as PostgreSQL, SQLite, and ClickHouse, but the current implementation phase only has a MySQL-compatible protocol target.
- TOML parsing rejects truly unknown protocol strings. Validation should reject known-but-currently-unsupported protocol adapters.

## Requirements

- Add validation support to `sql-lens-config`.
- Provide a public API on `SqlLensConfig` to validate a loaded or manually constructed config.
- Reject empty or whitespace-only `proxy.listen`.
- Reject empty or whitespace-only `backend.address`.
- Reject unsupported startup protocol adapters.
- Current supported startup protocol is `Protocol::MySql` only.
- Return structured validation errors suitable for CLI/runtime display later.
- Validation should collect all detected violations in one call, instead of stopping at the first violation.
- Validation errors should implement `Debug`, `Display`, and `std::error::Error`.
- Unit tests must cover valid defaults and each required violation.

## Out Of Scope

- Environment variable overrides.
- CLI `--config`.
- Runtime startup.
- Hot reload.
- File watching.
- Address syntax or socket address parsing.
- Checking whether ports are available.
- Storage capacity validation.
- TLS certificate path validation.
- Auth configuration validation.
- Protocol registry integration.
- Proxy, API, frontend, or logging initialization.

## Acceptance Criteria

- [ ] `SqlLensConfig::default().validate()` succeeds.
- [ ] Missing `proxy.listen` is rejected.
- [ ] Whitespace-only `proxy.listen` is rejected.
- [ ] Missing `backend.address` is rejected.
- [ ] Whitespace-only `backend.address` is rejected.
- [ ] Non-MySQL `proxy.protocol` is rejected as an unsupported adapter.
- [ ] Multiple validation violations can be returned together.
- [ ] Validation error implements `Display` and `std::error::Error`.
- [ ] Tests cover valid config and all validation failures.
- [ ] `cargo fmt --check` passes.
- [ ] `cargo check --workspace` passes.
- [ ] `cargo test --workspace` passes.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes.
- [ ] No env override, CLI, runtime startup, hot reload, watcher, storage, TLS, auth, proxy, API, frontend, or logging initialization logic is introduced.

## Open Questions

None blocking.
