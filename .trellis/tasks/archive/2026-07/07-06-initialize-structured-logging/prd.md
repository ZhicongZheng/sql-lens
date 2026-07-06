# Initialize Structured Logging

## Goal

Implement Issue 012: initialize structured logging for the `sql-lens` binary using the already-loaded runtime configuration.

## User Value

Developers should get predictable startup diagnostics before proxy, API, or runtime services exist. Logging should be machine-readable in JSON mode for tools and readable in pretty mode for local development.

## Background

- Issue 011 is complete: `sql-lens-app` parses `--config`, loads TOML through `sql-lens-config`, validates it, and exits with clear errors.
- `ARCHITECTURE.md` and `.trellis/spec/backend/directory-structure.md` assign logging setup to `sql-lens-app`.
- `CONFIG.md` defines `[logging]` with:
  - `level`
  - `format`
  - `redact_secrets`
- `sql-lens-config` already models:
  - `LoggingLevel::{Trace, Debug, Info, Warn, Error}`
  - `LoggingFormat::{Json, Pretty}`
  - `LoggingConfig { level, format, redact_secrets }`
- Context7 confirmed `tracing-subscriber` supports `fmt().json()`, `fmt().pretty()`, `with_max_level`, and `try_init`.

## Requirements

- Initialize logging in `sql-lens-app` after config load and config validation succeed.
- Use the loaded `config.logging.level` to set the maximum emitted log level.
- Use the loaded `config.logging.format` to select JSON or pretty formatter.
- Return a clear startup error if logging initialization fails.
- Emit a small startup-check log event after logging is initialized so smoke tests can verify formatter behavior.
- Keep logging initialization synchronous.
- Keep logging setup local to `sql-lens-app`; do not move config contracts into the app crate.
- Add tests or smoke checks for JSON format, pretty format, and configured log level behavior.

## Dependency Policy

- Allow adding `tracing` to `sql-lens-app`.
- Allow adding `tracing-subscriber` to `sql-lens-app` with the minimal features required for formatting and JSON output.
- Do not add async runtime dependencies.
- Do not add `env-filter`, `opentelemetry`, file appenders, log rotation, or external logging backends in this task.
- Do not add `assert_cmd`, `predicates`, `tempfile`, or `serde_json`; keep tests standard-library based.

## Out Of Scope

- Environment variable overrides.
- `RUST_LOG` / env filter support.
- Dynamic logging reload.
- File logging.
- OpenTelemetry exporter.
- Prometheus metrics.
- Request/connection spans.
- SQL redaction implementation.
- Proxy, API, web, storage, plugin, or runtime startup.
- Changing the existing `LoggingConfig` shape.

## Acceptance Criteria

- [x] `sql-lens --config <FILE>` initializes logging after config validation.
- [x] `logging.format = "json"` emits JSON-formatted startup log output.
- [x] `logging.format = "pretty"` emits human-readable startup log output.
- [x] `logging.level` controls whether the startup info log is emitted.
- [x] Logging initialization failures are represented in the app-level startup error type.
- [x] Existing config load and validation error behavior remains clear.
- [x] Tests or smoke checks cover JSON format.
- [x] Tests or smoke checks cover pretty format.
- [x] Tests or smoke checks cover configured log level filtering.
- [x] `cargo fmt --check` passes.
- [x] `cargo check --workspace` passes.
- [x] `cargo test --workspace` passes.
- [x] `cargo clippy --workspace --all-targets -- -D warnings` passes.
- [x] No proxy, API, storage, async runtime, env override, hot reload, or external exporter behavior is introduced.

## Open Questions

None blocking.
