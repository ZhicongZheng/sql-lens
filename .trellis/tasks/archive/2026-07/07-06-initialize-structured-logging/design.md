# Structured Logging Design

## Objective

Add the first real logging initialization path to `sql-lens-app` while preserving the current startup boundary:

1. Parse CLI args.
2. Load config.
3. Validate config.
4. Initialize logging from config.
5. Emit one startup-check log event.
6. Exit successfully.

No long-running runtime services should start in this task.

## Crate Boundary

Modify:

- `crates/sql-lens-app/Cargo.toml`
- `crates/sql-lens-app/src/main.rs`
- `crates/sql-lens-app/tests/cli.rs`

Expected lockfile update:

- `Cargo.lock`

Likely spec update:

- `.trellis/spec/backend/logging-guidelines.md`
- `.trellis/spec/backend/index.md`

Task metadata lives under:

- `.trellis/tasks/07-06-initialize-structured-logging/`

## Dependencies

Add to `sql-lens-app`:

```toml
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["json"] }
```

Rationale:

- `tracing` owns event macros such as `tracing::info!`.
- `tracing-subscriber` owns global subscriber installation and formatter selection.
- The `json` feature is required for `fmt().json()`.
- `env-filter` is intentionally excluded because this task requires config-driven level selection, not environment overrides.

## Startup Flow

```text
Cli::parse()
  -> SqlLensConfig::from_path(...)
  -> config.validate()
  -> init_logging(&config.logging)
  -> tracing::info!(...)
  -> ExitCode::SUCCESS
```

Errors flow through the existing app-level error wrapper:

```text
ConfigLoadError -> AppError::ConfigLoad -> stderr -> failure
ConfigValidationError -> AppError::ConfigValidation -> stderr -> failure
Logging init error -> AppError::LoggingInit -> stderr -> failure
```

## Logging Contract

Use `LoggingConfig` directly:

- `LoggingLevel::Trace` -> `LevelFilter::TRACE`
- `LoggingLevel::Debug` -> `LevelFilter::DEBUG`
- `LoggingLevel::Info` -> `LevelFilter::INFO`
- `LoggingLevel::Warn` -> `LevelFilter::WARN`
- `LoggingLevel::Error` -> `LevelFilter::ERROR`

Formatter selection:

- `LoggingFormat::Json` -> `tracing_subscriber::fmt().json()`
- `LoggingFormat::Pretty` -> `tracing_subscriber::fmt().pretty()`

Use `try_init`, not `init`, so initialization failure can be converted into a startup error.

Disable ANSI in pretty mode for deterministic test output unless implementation evidence shows this is unnecessary.

## Startup Check Event

Emit one info-level event after logging is initialized:

```text
SQL Lens startup checks completed
```

This event must not include config contents, credentials, SQL text, or user-provided database errors.

If `logging.level = "error"`, the info-level startup event should be filtered out. This provides a simple smoke check that the configured max level is active.

## Test Strategy

Use existing standard-library integration tests in `crates/sql-lens-app/tests/cli.rs`.

Add or update tests:

- JSON format: valid config with `[logging] level = "info", format = "json"` exits zero and stderr starts like JSON and includes the startup message.
- Pretty format: valid config with `[logging] level = "info", format = "pretty"` exits zero and stderr includes the startup message but is not JSON-shaped.
- Level filtering: valid config with `[logging] level = "error", format = "json"` exits zero and does not emit the info startup message.

Existing config load/validation failure tests should continue to assert clear stderr messages. Those failures occur before logging initializes.

## Compatibility

- Existing CLI flags remain unchanged.
- Existing `sql-lens-config` public API remains unchanged.
- The task does not change default config values.
- Future runtime tasks can add spans around listener startup, accepted sessions, backend dialing, and shutdown using this initialized subscriber.

## Risks

- `tracing_subscriber` global initialization can only happen once per process. Integration tests run the compiled binary in a child process, avoiding cross-test global subscriber conflicts.
- Logging to stderr means successful runs may now have stderr output when the configured level allows info logs. Existing tests should be updated to assert expected logging behavior instead of empty stderr.
- JSON output should be asserted with lightweight string checks to avoid adding `serde_json` only for tests.

## Rollback

Rollback by removing:

- `tracing` and `tracing-subscriber` dependencies from `sql-lens-app`.
- `init_logging` and startup log event from `main.rs`.
- Logging-specific integration test assertions.
- Lockfile changes introduced only by logging dependencies.
