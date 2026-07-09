# Logging Guidelines

> Concrete logging conventions for SQL Lens backend code.

## Scenario: Application Structured Logging

### 1. Scope / Trigger

- Trigger: `sql-lens-app` initializes process-wide logging for the `sql-lens` binary.
- Logging setup happens after config loading and validation, before any runtime service startup.
- Logging must help local debugging without exposing SQL text, credentials, authentication payloads, or database error contents before redaction rules exist.

### 2. Signatures

Startup logging is configured from `sql-lens-config`:

```rust
pub struct LoggingConfig {
    pub level: LoggingLevel,
    pub format: LoggingFormat,
    pub redact_secrets: bool,
}

pub enum LoggingLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

pub enum LoggingFormat {
    Json,
    Pretty,
}
```

`sql-lens-app` owns the initialization boundary:

```rust
fn init_logging(config: &LoggingConfig) -> Result<(), AppError>;
```

Allowed application logging dependencies:

```toml
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["json"] }
```

### 3. Contracts

- Log level comes from `config.logging.level`.
- Log format comes from `config.logging.format`.
- JSON format uses `tracing-subscriber` JSON formatter.
- Pretty format uses `tracing-subscriber` pretty formatter with ANSI disabled for deterministic CLI output.
- Logs are written to stderr, not stdout.
- Logging initialization must use `try_init` so startup can return a clear error.
- Startup emits one info-level smoke event after logging initialization:

```text
SQL Lens startup checks completed
```

- This startup event must not include config contents, SQL text, credentials, database errors, or authentication data.
- Runtime startup emits info-level lifecycle events for the bound API address
  and each proxy target listener address:

```text
SQL Lens API server listening
SQL Lens proxy target listening
```

- Proxy target startup logs may include `target_name`, `database_type`, and
  local listener address. They must not include backend credentials, SQL text,
  packet payloads, or unredacted database errors.
- Shutdown signal handling emits an info-level lifecycle event before graceful
  runtime shutdown:

```text
SQL Lens shutdown signal received
```

- `redact_secrets` remains part of the config contract, but this first logging layer must avoid logging secrets instead of trying to redact them.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| `logging.format = "json"` | Emit newline-delimited JSON log events to stderr |
| `logging.format = "pretty"` | Emit human-readable log events to stderr |
| `logging.level = "info"` | Emit the startup-check info event |
| `logging.level = "error"` | Filter out info-level startup, listener, and shutdown lifecycle events |
| Global subscriber cannot be installed | Return app-level logging initialization error; exit non-zero |
| Config loading or validation fails first | Keep existing stderr error behavior; do not initialize logging |

### 5. Good/Base/Bad Cases

Good:

- `tracing::info!("SQL Lens startup checks completed")` after config validation and logging initialization.
- Logging lifecycle messages such as startup, API bind, target listener bind,
  shutdown signal, and graceful stop.
- Structured fields with low-sensitivity values such as component names,
  target names, database type labels, and local bind addresses after those
  tasks define their contracts.

Base:

- Integration tests run the compiled `sql-lens` binary and inspect stderr.
- JSON tests use lightweight string checks instead of adding JSON parsing dependencies only for smoke tests.

Bad:

- Logging authentication payloads, passwords, SQL parameters, raw SQL text, or backend error messages before redaction is implemented.
- Writing logs to stdout in the CLI.
- Initializing logging inside `sql-lens-config`.
- Adding `RUST_LOG`, `EnvFilter`, OpenTelemetry, file logging, or log rotation without a task-level design update.

### 6. Tests Required

For logging startup changes:

- JSON format smoke test: exit success and stderr contains JSON-shaped startup log output.
- Pretty format smoke test: exit success and stderr contains the startup message without JSON shape.
- Level filtering smoke test: `error` level suppresses the info startup event.
- Existing config load and validation failure tests still pass.
- Run `cargo fmt --check`.
- Run `cargo check --workspace`.
- Run `cargo test --workspace`.
- Run `cargo clippy --workspace --all-targets -- -D warnings`.

### 7. Wrong vs Correct

#### Wrong

```rust
println!("loaded config: {config:?}");
tracing_subscriber::fmt().init();
```

#### Correct

```rust
fn run(cli: Cli) -> Result<(), AppError> {
    let config = SqlLensConfig::from_path(&cli.config)?;
    config.validate()?;
    init_logging(&config.logging)?;
    tracing::info!("SQL Lens startup checks completed");
    Ok(())
}
```

## Log Levels

- `trace`: temporary local protocol or packet investigation only. Do not log secrets or packet payloads.
- `debug`: development diagnostics that explain startup or lifecycle decisions.
- `info`: normal lifecycle milestones such as startup checks, listener started, and graceful shutdown.
- `warn`: recoverable problems that may affect debugging quality.
- `error`: startup failures and unrecoverable service failures.

## What Not To Log

- Passwords, tokens, secrets, authentication packets, private keys, TLS material, SQL parameters, raw SQL text, and unredacted database error text.
- Full config structs, because future config fields may contain sensitive paths or credentials.
