# Configuration Model

## Goal

Implement the first SQL Lens configuration model in a new independent crate named `sql-lens-config`.

This task turns the configuration contract from `CONFIG.md` into Rust structs, enums, defaults, and lightweight tests. It does not load TOML files, validate config, process environment overrides, implement hot reload, or parse CLI flags.

## User Value

SQL Lens needs a typed configuration contract before runtime startup, config loading, validation, CLI integration, proxy startup, and future hot reload can be implemented safely.

## Background

The Rust workspace skeleton and `sql-lens-core` domain models are complete.

Repository evidence:

- `MILESTONE.md` Milestone 2 is "Configuration And Runtime Startup".
- `ISSUES.md` Issue 007 is "Add configuration crate module".
- `ISSUES.md` Issue 008, 009, 010, and 011 separately cover TOML loading, environment overrides, validation, and CLI entry point.
- `CONFIG.md` defines the intended configuration sections, fields, and default example values.
- `ARCHITECTURE.md` says `sql-lens-app` owns CLI, config loading, logging setup, runtime startup, and graceful shutdown.
- `README.md` documents a future `sql-lens --config sql-lens.toml` flow, but CLI behavior is Issue 011 and remains out of scope.

## Scope

Add a new workspace member crate:

- `crates/sql-lens-config`

Implement:

- Configuration structs.
- Configuration enums.
- Explicit defaults.
- Lightweight unit tests for default config construction and key default values.

Include all sections documented in `CONFIG.md`:

- `proxy`
- `backend`
- `tls`
- `web`
- `storage`
- `retention`
- `logging`
- `redaction`
- `auth`
- `replay`
- `plugins`

## Technical Decisions

- Configuration model lives in `sql-lens-config`.
- Do not put configuration model types in `sql-lens-app`.
- Do not put runtime configuration types in `sql-lens-core`.
- `sql-lens-config` depends on `serde` only:
  - `serde = { version = "1.0", features = ["derive"] }`
- Do not add `toml`.
- Do not add validation crates.
- Do not depend on `sql-lens-core`.
- Use config-owned enums for:
  - protocol
  - database type
  - TLS mode
  - storage type
  - logging level
  - logging format
  - auth mode
  - retention drop policy
- Long-term configuration source strategy is hybrid:
  - TOML for startup and foundational system configuration.
  - SQLite for future runtime settings mutated through UI/API.
  - Hot reload as a separate mechanism layered on top of config loading and runtime apply rules.
- Do not add field-level reloadability metadata in this first model slice.

## Required Defaults

Defaults should match `CONFIG.md` where specified:

- `proxy.listen = "127.0.0.1:3307"`
- `proxy.protocol = "mysql"`
- `proxy.capture_mode = "observe"`
- `proxy.max_connections = 512`
- `proxy.connect_timeout_ms = 5000`
- `proxy.idle_timeout_ms = 300000`
- `backend.address = "127.0.0.1:3306"`
- `backend.database_type = "mysql"`
- `tls.mode = "passthrough"`
- `web.listen = "127.0.0.1:5173"`
- `web.base_url = "http://127.0.0.1:5173"`
- `web.cors_origins = ["http://127.0.0.1:5173"]`
- `storage.type = "ring_buffer"`
- `storage.capacity = 100000`
- `retention.max_age = "24h"`
- `retention.max_events = 100000`
- `logging.level = "info"`
- `logging.format = "json"`
- `logging.redact_secrets = true`
- `redaction.enabled = true`
- `redaction.mask = "***"`
- `redaction.parameter_names = ["password", "token", "secret"]`
- `auth.enabled = false`
- `auth.mode = "local"`
- `auth.session_ttl = "12h"`
- `replay.enabled = true`
- `replay.require_confirmation_for_mutations = true`
- `plugins.enabled = false`
- `plugins.directory = "plugins"`

## Out Of Scope

- TOML file loading.
- Environment variable overrides.
- Config validation.
- CLI `--config`.
- Runtime startup.
- Hot reload.
- SQLite runtime settings storage.
- Field-level reloadability metadata.
- Logging setup.
- Proxy startup.
- Web/API startup.

## Acceptance Criteria

- [ ] Root workspace includes `crates/sql-lens-config` as a member.
- [ ] `sql-lens-config` crate exists and builds.
- [ ] `sql-lens-config` depends on `serde` with derive support.
- [ ] `sql-lens-config` does not depend on `toml`, validation crates, or `sql-lens-core`.
- [ ] All `CONFIG.md` sections are represented by typed structs.
- [ ] Config-owned enums represent known option sets.
- [ ] `SqlLensConfig::default()` or equivalent top-level default exists.
- [ ] Unit tests cover default construction and key default values.
- [ ] `cargo fmt --check` passes.
- [ ] `cargo check --workspace` passes.
- [ ] `cargo test --workspace` passes.
- [ ] No TOML loading, env override, validation, CLI, hot reload, SQLite settings, proxy, API, or frontend logic is introduced.

## Open Questions

None blocking.
