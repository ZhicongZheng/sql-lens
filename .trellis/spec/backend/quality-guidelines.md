# Backend Quality Guidelines

> Code quality standards for backend development.

## Scenario: Core Domain Model Contracts

### 1. Scope / Trigger

- Trigger: `sql-lens-core` defines cross-layer public contract types used by proxy, protocol adapters, storage, API, WebSocket, plugins, and UI-facing schemas.
- These types are not implementation details. Changes to them can ripple across multiple layers.
- Core models must stay protocol-neutral. Protocol-specific fields belong in typed metadata.

### 2. Signatures

Core model modules live under:

```text
crates/sql-lens-core/src/
  ids.rs
  time.rs
  metadata.rs
  event.rs
  error.rs
```

Public types are re-exported from `lib.rs`.

Required dependency policy for the first core model layer:

```toml
serde = { version = "1.0", features = ["derive"] }
```

Do not add these dependencies to `sql-lens-core` without a new design decision:

- `serde_json`
- `time`
- `uuid`
- async runtime crates
- HTTP framework crates
- database or storage crates

### 3. Contracts

Public model types should derive:

- `Debug`
- `Clone`
- `PartialEq`
- `serde::Serialize`
- `serde::Deserialize`
- `Eq` only where practical

Do not force `Eq` onto types containing floating-point values or metadata that can contain floating-point values.

ID and time values must use core-owned newtypes:

- `SqlEventId`
- `ConnectionId`
- `StatementId`
- `RequestId`
- `Timestamp`
- `DurationMillis`

Protocol metadata must use typed fields:

```rust
pub struct ProtocolMetadata {
    pub protocol: ProtocolName,
    pub fields: Vec<MetadataField>,
}
```

Do not use arbitrary JSON for protocol metadata in the first core contract.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| A shared model needs protocol-specific data | Put it under `ProtocolMetadata`, not as a top-level field |
| A model contains `f64` directly or indirectly | Do not derive `Eq` |
| A new public model is added | Re-export it from `lib.rs` and add a construction or trait test |
| A new dependency is proposed for `sql-lens-core` | Justify it in the task design before implementation |
| API-style errors are needed | Use `ApiError` and `ApiErrorCode`; do not invent per-layer response shapes |

### 5. Good/Base/Bad Cases

Good:

- `SqlEvent` uses `ProtocolName`, `DatabaseType`, `ConnectionId`, `Timestamp`, and `ProtocolMetadata`.
- MySQL statement IDs are represented inside metadata or a protocol-neutral statement ID wrapper.
- `ApiErrorCode` is shared and stable.

Base:

- A new optional field is added to `SqlEvent` only after checking `PRD.md`, `STORAGE.md`, and `API.md`.
- A new enum variant is added with tests and downstream impact noted.

Bad:

- Adding `mysql_statement_id: u32` as a top-level `SqlEvent` field.
- Adding `serde_json::Value` to `ProtocolMetadata` without a design update.
- Deriving `Eq` for a type that contains `f64`.

### 6. Tests Required

For public core model changes:

- Construct representative instances in unit tests.
- Assert important fields.
- Add compile-time trait checks for `Serialize` and `Deserialize`.
- Run `cargo fmt --check`.
- Run `cargo check --workspace`.
- Run `cargo test --workspace`.
- Run `cargo clippy --workspace --all-targets -- -D warnings` before committing backend model changes.

### 7. Wrong vs Correct

#### Wrong

```rust
pub struct SqlEvent {
    pub mysql_statement_id: Option<u32>,
    pub metadata: serde_json::Value,
}
```

#### Correct

```rust
pub struct SqlEvent {
    pub metadata: ProtocolMetadata,
}

pub struct ProtocolMetadata {
    pub protocol: ProtocolName,
    pub fields: Vec<MetadataField>,
}
```

## Scenario: Config Loading Contracts

### 1. Scope / Trigger

- Trigger: `sql-lens-config` owns startup configuration structs, serde-compatible config shape, default values, startup TOML parsing, and local environment-variable overrides.
- Config loading is a boundary contract for CLI startup, validation, logging setup, and runtime startup.
- Config parsing must stay separate from semantic validation, runtime apply logic, and environment override application.

### 2. Signatures

Public TOML loading APIs live on `SqlLensConfig`:

```rust
impl SqlLensConfig {
    pub fn from_path(path: impl AsRef<std::path::Path>) -> Result<Self, ConfigLoadError>;
    pub fn from_toml_str(input: &str) -> Result<Self, ConfigLoadError>;
    pub fn apply_env_overrides(&mut self) -> Result<(), ConfigOverrideError>;
    pub fn apply_env_overrides_from<I, K, V>(&mut self, variables: I) -> Result<(), ConfigOverrideError>
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str>,
        V: AsRef<str>;
}
```

Structured load errors use:

```rust
pub enum ConfigLoadError {
    Read {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    Parse {
        path: Option<std::path::PathBuf>,
        source: toml::de::Error,
    },
}

pub struct ConfigOverrideError {
    pub variable: String,
    pub value: String,
    pub expected: &'static str,
}
```

### 3. Contracts

- `from_path` reads a file and parses it as TOML.
- `from_toml_str` parses already-loaded TOML content.
- Missing sections and fields use the existing `Default` implementations.
- Unknown sections and fields are rejected with `#[serde(deny_unknown_fields)]`.
- Environment overrides are applied explicitly after TOML parsing and before
  validation/runtime startup.
- `apply_env_overrides_from` exists so unit tests can avoid mutating
  process-global environment variables.
- Supported overrides are `SQL_LENS_PROXY_LISTEN`,
  `SQL_LENS_BACKEND_ADDRESS`, and `SQL_LENS_LOGGING_LEVEL`.
- Legacy proxy/backend overrides do not rewrite explicit `[[targets]]` entries.
- The config crate may depend on `serde` and `toml` for this layer.
- The config crate must not depend on CLI, async runtime, HTTP, database, watcher, or proxy crates for loading.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| Config file cannot be read | Return `ConfigLoadError::Read` with the path and IO source |
| TOML from a path cannot be parsed | Return `ConfigLoadError::Parse` with `Some(path)` |
| TOML from a string cannot be parsed | Return `ConfigLoadError::Parse` with `None` |
| Section or field is missing | Use the section or field default |
| Section or field is unknown | Return a parse/deserialization error |
| Supported env override is present | Mutate the matching config field before validation |
| `SQL_LENS_LOGGING_LEVEL` is invalid | Return `ConfigOverrideError` with expected values |
| Unknown `SQL_LENS_*` override is present | Ignore it |
| `[[targets]]` is present | Do not rewrite target entries from legacy proxy/backend env overrides |
| Required semantic value is empty or unsupported at runtime | Leave to the later config validation layer |

### 5. Good/Base/Bad Cases

Good:

- A local config file contains only `[proxy] listen = "127.0.0.1:3308"` and the rest comes from defaults.
- A misspelled field like `lissten` fails during TOML deserialization.
- A missing file reports `ConfigLoadError::Read`.
- `SQL_LENS_LOGGING_LEVEL=debug` updates `config.logging.level` after TOML parsing.

Base:

- A caller uses `SqlLensConfig::from_toml_str` in tests and `SqlLensConfig::from_path` in CLI code.
- CLI startup calls `config.apply_env_overrides()` before `config.validate()`.
- Validation later rejects semantically invalid values after TOML parsing succeeds.

Bad:

- Silently ignoring unknown config fields.
- Starting services from `sql-lens-config`.
- Reading environment variables inside `from_path` or `from_toml_str`.
- Adding hot reload or CLI argument parsing inside `sql-lens-config`.

### 6. Tests Required

For config loading changes:

- Valid TOML file loads from a path.
- TOML string parsing works.
- Partial TOML falls back to defaults.
- Unknown top-level sections and nested fields fail.
- Invalid TOML returns `ConfigLoadError::Parse`.
- Missing files return `ConfigLoadError::Read`.
- `ConfigLoadError` implements `Display` and `std::error::Error`.
- Supported env overrides update the expected fields.
- Invalid logging-level env override returns `ConfigOverrideError`.
- `ConfigOverrideError` implements `Display` and `std::error::Error`.

### 7. Wrong vs Correct

#### Wrong

```rust
pub fn load_config(path: &str) -> SqlLensConfig {
    toml::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}
```

#### Correct

```rust
pub fn load_config(path: impl AsRef<std::path::Path>) -> Result<SqlLensConfig, ConfigLoadError> {
    SqlLensConfig::from_path(path)
}
```

## Scenario: MySQL Prepared Statement Capture Events

### 1. Scope / Trigger

- Trigger: `sql-lens-protocol-mysql` turns `COM_STMT_EXECUTE` traffic into
  cross-layer `SqlEvent` records consumed by storage, REST API, WebSocket, and
  future UI surfaces.
- Prepared statement execution events are public debugging data. MySQL-only
  details must stay in protocol metadata, not top-level core fields.

### 2. Signatures

Prepared execute events use the existing core model:

```rust
SqlEvent {
    kind: SqlEventKind::StatementExecute,
    protocol: ProtocolName("mysql".to_owned()),
    original_sql: "<prepared template SQL>".to_owned(),
    expanded_sql: Some("<rendered SQL with parameters>".to_owned()),
    parameters: Vec<SqlParameter>,
    metadata: ProtocolMetadata { protocol, fields },
    ..
}
```

Required MySQL metadata fields:

- `command = "COM_STMT_EXECUTE"`
- `command_sequence_id`
- `statement_id`
- `flags`
- `iteration_count`
- `ok_status_flags` when present on OK packets

### 3. Contracts

- Client `COM_STMT_EXECUTE` observation may store decoded parameters and
  expanded SQL on MySQL connection-local state.
- Backend OK/ERR terminal packets complete that stored execute envelope and
  emit exactly one `SqlEventKind::StatementExecute` event.
- The event's `original_sql` is the prepared template SQL when the statement ID
  is known.
- The event's `expanded_sql` comes from the MySQL execute renderer.
- Storage redaction remains the owner of masking before API exposure. Protocol
  code may provide parameter names inferred from safe template context such as
  `password = ?`, but must not implement a second redaction policy.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| Execute envelope exists and backend OK arrives | Emit `StatementExecute` with `status = Ok`, result summary, parameters, expanded SQL, and metadata |
| Execute envelope exists and backend ERR arrives | Emit `StatementExecute` with `status = Error` and sanitized `ErrorSummary` |
| Backend response is unsupported or incomplete | Keep the envelope and return without emitting |
| Client execute packet is malformed or parameter decoding fails | Stay non-fatal and do not create an execute envelope |
| Sensitive parameter name matches default policy | Ring buffer stores/API returns masked parameter value and masked expanded SQL |

### 5. Good/Base/Bad Cases

Good:

- `UPDATE users SET name = ?, password = ? WHERE id = 42` emits a
  `statement_execute` API detail response with `password` redacted by storage.

Base:

- Unknown statement IDs may still produce a MySQL execute envelope, but the
  event should not invent template SQL or parameters.

Bad:

- Adding `mysql_statement_id` directly to `SqlEvent`.
- Redacting values inside the MySQL adapter instead of letting storage apply the
  configured redaction policy.
- Dropping prepared execute events because resultset capture is not implemented;
  OK/ERR terminal paths should still be captured.

### 6. Tests Required

- Protocol unit test: prepared execute + backend OK emits
  `SqlEventKind::StatementExecute` with template SQL, decoded parameters,
  expanded SQL, result summary, and MySQL metadata.
- Docker integration test: real MySQL driver prepared statement through the
  proxy appears in `GET /api/v1/sql-events` and detail endpoint.
- Redaction assertion: API-visible sensitive parameter value and expanded SQL
  are masked after storage retention.
- Regression checks: existing COM_QUERY live coverage and MySQL protocol unit
  tests remain green.

### 7. Wrong vs Correct

#### Wrong

```rust
SqlEvent {
    kind: SqlEventKind::Query,
    original_sql: expanded_sql,
    parameters: Vec::new(),
    metadata: ProtocolMetadata { fields: Vec::new(), .. },
    ..
}
```

#### Correct

```rust
SqlEvent {
    kind: SqlEventKind::StatementExecute,
    original_sql: template_sql,
    expanded_sql: Some(expanded_sql),
    parameters,
    metadata: mysql_execute_metadata,
    ..
}
```

## Scenario: MySQL-Compatible Docker Smoke Tests

### 1. Scope / Trigger

- Trigger: `sql-lens-app` integration tests start live MySQL-compatible
  databases in Docker and verify traffic through the SQL Lens proxy plus REST
  API state.
- These tests prove runtime wiring across app, proxy, MySQL protocol adapter,
  capture storage, and API. They must stay opt-in because the containers are
  slow and require Docker access.

### 2. Signatures

Environment gates are one variable per live target:

```text
SQL_LENS_DOCKER_TESTS=1
SQL_LENS_STARROCKS_TESTS=1
SQL_LENS_TIDB_TESTS=1
SQL_LENS_DORIS_TESTS=1
```

Shared test URLs must disable socket preference:

```text
mysql://<user>[:password]@<host>:<port>[/database]?prefer_socket=false
```

### 3. Contracts

- Default `cargo test` and `cargo test --workspace` must skip Docker smoke
  tests unless the target environment gate is set.
- Smoke tests connect through `start_minimal_mysql_runtime`, not directly to
  the backend for the captured query path.
- API assertions use `GET /api/v1/sql-events` and, when parameters matter,
  `GET /api/v1/sql-events/{id}`.
- Prepared statement smoke coverage should follow the existing MySQL/TiDB
  `statement_execute` API shape: template SQL in `original_sql`, rendered SQL
  in `expanded_sql`, and sensitive parameters redacted by storage/API.
- Docker-mapped localhost connections must set `prefer_socket=false`; otherwise
  `mysql_async` may query `@@socket`, which is unsupported by some
  MySQL-compatible targets such as StarRocks.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| Environment gate is unset | Test prints a skip message and returns success |
| Backend port opens before query readiness | Keep polling a small `SELECT 1` readiness query until timeout |
| MySQL-compatible target lacks `@@socket` | Use `prefer_socket=false`; do not add product runtime workarounds |
| Captured event does not appear in API | Fail the smoke test with a timeout |
| Target prepared statements are outside first-smoke scope | Document the gap in the task or a follow-up issue |

### 5. Good/Base/Bad Cases

Good:

- A TiDB smoke test runs text query and prepared execute through SQL Lens, then
  asserts API summary and detail fields.

Base:

- StarRocks and Doris first-smoke tests run stable text queries through SQL
  Lens and assert API capture, while documenting prepared statement gaps.

Bad:

- Running Docker containers in default test execution.
- Connecting directly to the backend for the query that should be captured.
- Removing `prefer_socket=false` from localhost Docker test URLs.

### 6. Tests Required

- `rtk cargo fmt --check`.
- `rtk cargo test -p sql-lens-app`.
- Target-specific env-gated Docker smoke command for each new live target.
- Existing MySQL Docker smoke command when shared helpers change.
- `rtk cargo test --workspace`.
- `rtk cargo clippy --workspace --all-targets -- -D warnings`.

### 7. Wrong vs Correct

#### Wrong

```rust
let url = format!("mysql://root@{address}");
```

#### Correct

```rust
let url = format!("mysql://root@{address}?prefer_socket=false");
```

## Scenario: Config Validation Contracts

### 1. Scope / Trigger

- Trigger: `sql-lens-config` validates semantic startup readiness after TOML loading and before CLI/runtime startup.
- Validation must stay separate from TOML parsing, environment overrides, and service startup.
- Validation errors are public contracts for later CLI and runtime error display.

### 2. Signatures

Public validation API:

```rust
impl SqlLensConfig {
    pub fn validate(&self) -> Result<(), ConfigValidationError>;
}
```

Structured validation errors:

```rust
pub struct ConfigValidationError {
    pub violations: Vec<ConfigValidationViolation>,
}

pub enum ConfigValidationViolation {
    MissingProxyListen,
    MissingBackendAddress,
    UnsupportedProtocol { protocol: Protocol },
}
```

### 3. Contracts

- `SqlLensConfig::default().validate()` must succeed.
- Validation collects all detected violations instead of failing fast.
- Empty and whitespace-only required string fields are treated as missing.
- The current supported startup protocol is `Protocol::MySql`.
- Future protocol enum variants may deserialize, but validation rejects them until their adapters exist.
- Address syntax parsing, port availability checks, TLS certificate checks, and auth checks are outside the first validation layer.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| `proxy.listen.trim().is_empty()` | Add `MissingProxyListen` |
| `backend.address.trim().is_empty()` | Add `MissingBackendAddress` |
| `proxy.protocol != Protocol::MySql` | Add `UnsupportedProtocol { protocol }` |
| Multiple violations exist | Return all in one `ConfigValidationError` |
| No violations exist | Return `Ok(())` |

### 5. Good/Base/Bad Cases

Good:

- CLI later calls `SqlLensConfig::from_path(path)?.validate()?` before runtime startup.
- A config with missing listen address and unsupported protocol reports both issues.

Base:

- TOML parsing rejects unknown fields before validation runs.
- Validation rejects known-but-currently-unsupported protocol variants such as `postgresql`.

Bad:

- Duplicating validation logic in CLI or runtime startup.
- Starting services and discovering missing required fields later.
- Doing socket bind tests or network probes inside `sql-lens-config`.

### 6. Tests Required

For config validation changes:

- Default config validates successfully.
- Empty and whitespace-only `proxy.listen` fail.
- Empty and whitespace-only `backend.address` fail.
- Non-MySQL `proxy.protocol` fails.
- Multiple violations are returned together and in deterministic order.
- `ConfigValidationError` implements `Display` and `std::error::Error`.

### 7. Wrong vs Correct

#### Wrong

```rust
if config.proxy.listen.is_empty() {
    panic!("missing proxy listen");
}
```

#### Correct

```rust
config.validate()?;
```

## Scenario: CLI Entry Point Contracts

### 1. Scope / Trigger

- Trigger: `sql-lens-app` owns the user-facing `sql-lens` binary.
- The CLI is the startup contract for local development, CI smoke tests, and
  the local demo runtime.
- Runtime startup belongs in `sql-lens-app`; config loading and validation
  remain owned by `sql-lens-config`, and HTTP serving remains owned by
  `sql-lens-api`.

### 2. Signatures

The initial command surface is:

```text
sql-lens [--config <FILE>]
sql-lens --version
sql-lens --help
```

The Rust entry point shape should stay small:

```rust
#[tokio::main(flavor = "current_thread")]
async fn main() -> std::process::ExitCode;
async fn run(cli: Cli) -> Result<(), AppError>;
async fn wait_for_shutdown_signal() -> Result<(), AppError>;
```

Runtime composition is exposed from the app library for tests and future
composition:

```rust
pub async fn start_runtime_from_config(
    config: &sql_lens_config::SqlLensConfig,
) -> Result<MinimalMysqlRuntime, MinimalMysqlRuntimeError>;
```

Required application startup dependencies include:

```toml
clap = { version = "4", features = ["derive"] }
sql-lens-config = { path = "../sql-lens-config" }
sql-lens-storage = { path = "../sql-lens-storage" }
tokio = { version = "1", features = ["macros", "rt", "signal"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["json"] }
```

### 3. Contracts

- `--config <FILE>` loads the selected TOML file through `SqlLensConfig::from_path`.
- The default config path is `sql-lens.toml`.
- The loaded config is validated through `SqlLensConfig::validate`.
- `--version` is handled by clap and exits successfully without loading config.
- Successful load, validation, and logging initialization start the configured
  runtime and keep the process alive until a shutdown signal arrives.
- Runtime startup uses every `SqlLensConfig::effective_targets()` entry:
  explicit `[[targets]]` when present, otherwise the legacy `[proxy]` +
  `[backend]` pair.
- Runtime startup binds the API server to `web.listen`.
- Runtime startup creates one shared `ApiState` for REST handlers, WebSocket
  broadcast, live statistics, and ring-buffer event storage.
- When `storage.type = "sqlite"`, runtime startup opens/migrates
  `storage.path` through `SqliteEventStore::open` and starts a bounded
  persistence worker. REST SQL event timeline/detail/export endpoints and
  replay preview event lookup read persisted events through the configured
  SQLite API read source.
- The CLI owns OS signal handling; `sql-lens-api` owns HTTP graceful shutdown
  primitives and `sql-lens-proxy` owns listener/session primitives.
- Ctrl-C triggers graceful shutdown of the API server and proxy listeners.
- Config load or validation failure prints a human-readable message to stderr and exits with `ExitCode::FAILURE`.
- Logging initialization happens after config validation; follow `logging-guidelines.md`.
- Runtime startup failures are wrapped in `AppError` and exit with
  `ExitCode::FAILURE`.
- Do not add config hot reload, frontend static serving, TLS termination,
  replay execute, or new storage backends in the CLI runtime startup path
  without a dedicated task.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| `--version` is passed | Print version and exit zero before config loading |
| `--config <FILE>` points to a valid config | Load, validate, initialize logging, bind API/proxy listeners, and wait for shutdown |
| Config file cannot be read | Include config load context and the path in stderr; exit non-zero |
| TOML cannot be parsed | Include config load context and parse error in stderr; exit non-zero |
| Config validation fails | Include validation context and violation details in stderr; exit non-zero |
| Running without `--config` | Attempt to load `sql-lens.toml` |
| `web.listen` cannot bind | Return a runtime startup error and exit non-zero |
| Any proxy target listen address cannot bind | Return a runtime startup error and exit non-zero |
| `storage.type = "sqlite"` and `storage.path` is empty | Return a runtime startup error and exit non-zero |
| SQLite storage cannot be opened or migrated | Return a runtime startup error and exit non-zero |
| SQLite per-event persistence fails after startup | Log a warning and keep proxy forwarding alive |
| Ctrl-C is received | Stop API server and proxy listeners before returning success |

### 5. Good/Base/Bad Cases

Good:

- CLI code delegates parsing to clap and delegates config semantics to `sql-lens-config`.
- App-level errors wrap config errors only to add startup context.
- Runtime startup converts config into `HttpServerConfig` and
  `MinimalMysqlTargetConfig` instead of duplicating config validation rules.
- Tests use `127.0.0.1:0` or preallocated loopback ports to avoid port
  collisions.

Base:

- Integration tests run the compiled `sql-lens` binary with standard library `Command`.
- Test configs use temporary files and explicit `--config` paths.
- Binary startup tests poll `/api/v1/health` to prove the process is serving
  before cleaning up the child process.

Bad:

- Duplicating config validation rules in `sql-lens-app`.
- Calling `unwrap` or `expect` on user-provided config load/validation paths.
- Starting services from `sql-lens-config` or `sql-lens-api`.
- Blocking packet forwarding on REST, WebSocket, SQLite persistence, plugins,
  or frontend work.
- Adding hot reload, static frontend hosting, replay execute, or new
  storage backends to the CLI runtime startup task.

### 6. Tests Required

For CLI entry point changes:

- `--version` exits successfully and includes the package version.
- `--config <valid-file>` starts the API server and responds to
  `/api/v1/health`.
- Runtime-from-config tests bind configured `web.listen` and all effective
  proxy targets using ephemeral or preallocated loopback ports.
- Runtime shutdown tests exercise a deterministic shutdown primitive without
  requiring a live database.
- Missing config path exits non-zero and stderr includes load/read context.
- Invalid config exits non-zero and stderr includes validation context and violation fields.
- Run `cargo fmt --check`.
- Run `cargo check --workspace`.
- Run `cargo test --workspace`.
- Run `cargo clippy --workspace --all-targets -- -D warnings`.

### 7. Wrong vs Correct

#### Wrong

```rust
fn run(cli: Cli) -> Result<(), AppError> {
    let config = SqlLensConfig::from_path(&cli.config)?;
    config.validate()?;
    Ok(())
}
```

#### Correct

```rust
async fn run(cli: Cli) -> Result<(), AppError> {
    let config = SqlLensConfig::from_path(&cli.config)?;
    config.validate()?;
    init_logging(&config.logging)?;
    let runtime = start_runtime_from_config(&config).await?;
    wait_for_shutdown_signal().await?;
    runtime.shutdown().await?;
    Ok(())
}
```
- Run `cargo clippy --workspace --all-targets -- -D warnings`.

### 7. Wrong vs Correct

#### Wrong

```rust
fn main() {
    let config = SqlLensConfig::from_path("sql-lens.toml").unwrap();
    if config.proxy.listen.is_empty() {
        panic!("missing proxy listen");
    }
}
```

#### Correct

```rust
fn run(cli: Cli) -> Result<(), AppError> {
    let config = SqlLensConfig::from_path(&cli.config)?;
    config.validate()?;
    Ok(())
}
```

## Scenario: TCP Proxy Listener Contracts

### 1. Scope / Trigger

- Trigger: `sql-lens-proxy` owns the first network listener boundary for database client connections.
- The listener layer binds local TCP sockets, accepts clients, and hands accepted sockets to later session logic.
- This layer must not dial backends, forward bytes, parse protocols, emit capture events, or start the application runtime.

### 2. Signatures

Public listener types live in `crates/sql-lens-proxy/src/lib.rs`:

```rust
pub struct ProxyListenerConfig {
    pub listen: String,
}

pub struct TcpProxyListener;

impl TcpProxyListener {
    pub async fn bind(config: ProxyListenerConfig) -> Result<Self, ProxyListenerError>;
    pub fn local_addr(&self) -> Result<std::net::SocketAddr, ProxyListenerError>;
    pub async fn accept(&self) -> Result<AcceptedClient, ProxyListenerError>;
    pub async fn run_accept_loop(
        self,
        accepted_tx: tokio::sync::mpsc::Sender<AcceptedClient>,
        shutdown: tokio::sync::watch::Receiver<bool>,
    ) -> Result<AcceptLoopStats, ProxyListenerError>;
}
```

Allowed listener dependencies:

```toml
tokio = { version = "1", features = ["net", "sync", "time", "rt", "macros"] }
tracing = "0.1"
```

Do not add `tokio-util`, `thiserror`, `anyhow`, `async-trait`, backend client libraries, protocol crates, or app composition dependencies for the first listener boundary.

### 3. Contracts

- `ProxyListenerConfig.listen` is the runtime bind address string.
- `TcpProxyListener::bind` binds a Tokio `TcpListener`.
- `TcpProxyListener::local_addr` returns the actual bound local address, including OS-assigned ports from `127.0.0.1:0`.
- `TcpProxyListener::accept` returns an `AcceptedClient` with the peer address and owned client stream.
- `TcpProxyListener::run_accept_loop` sends accepted clients through an `mpsc::Sender<AcceptedClient>`.
- Shutdown is represented by `tokio::sync::watch::Receiver<bool>`.
- `shutdown = true` stops accepting new client sockets.
- Dropping the shutdown sender also stops the accept loop.
- `AcceptLoopStats.accepted_connections` reports how many accepted clients were delivered before the loop stopped.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| Listener bind succeeds | Return `TcpProxyListener` |
| Listener bind fails | Return `ProxyListenerError::Bind { listen, source }` |
| Local address lookup fails | Return `ProxyListenerError::LocalAddr { source }` |
| Accept fails | Return `ProxyListenerError::Accept { source }` |
| Accepted-client receiver is closed | Return `ProxyListenerError::AcceptedClientReceiverClosed` |
| Shutdown receiver changes to `true` | Stop loop and return stats |
| Shutdown sender is dropped | Stop loop and return stats |

### 5. Good/Base/Bad Cases

Good:

- Bind `127.0.0.1:0` in tests and inspect `local_addr`.
- Use `watch::Receiver<bool>` for the first simple shutdown boundary.
- Use `mpsc::Sender<AcceptedClient>` as the handoff point to future session/backend dialing work.

Base:

- The accept loop owns the listener and returns stats when it stops.
- Tests use `tokio::time::timeout` around async accept-loop joins so failures do not hang.
- Test-only client connections may use `TcpStream::connect` to exercise accepting behavior.

Bad:

- Calling backend `connect` from listener code.
- Adding byte forwarding or protocol parsing to `sql-lens-proxy` listener tests.
- Importing `sql-lens-app`, `sql-lens-config`, protocol crates, storage crates, or API crates into `sql-lens-proxy` for listener work.
- Using fixed ports in tests except when intentionally testing a second-bind failure against an already-bound local address.

### 6. Tests Required

For TCP listener changes:

- Successful bind test using `127.0.0.1:0`.
- Structured bind failure test using a second bind to an already-bound address.
- Accept-loop delivery test that connects a local client and receives an `AcceptedClient`.
- Shutdown test that stops the accept loop without a client connection.
- Run socket-binding tests outside sandboxes that deny local TCP binds.
- Run `cargo fmt --check`.
- Run `cargo check --workspace`.
- Run `cargo test --workspace`.
- Run `cargo clippy --workspace --all-targets -- -D warnings`.

### 7. Wrong vs Correct

#### Wrong

```rust
pub async fn run_proxy(listen: &str, backend: &str) -> std::io::Result<()> {
    let listener = TcpListener::bind(listen).await?;
    let (client, _) = listener.accept().await?;
    let backend = TcpStream::connect(backend).await?;
    tokio::io::copy_bidirectional(&mut client, &mut backend).await?;
    Ok(())
}
```

#### Correct

```rust
let listener = TcpProxyListener::bind(ProxyListenerConfig::new("127.0.0.1:0")).await?;
let stats = listener.run_accept_loop(accepted_tx, shutdown_rx).await?;
```

## Scenario: Backend Dialing Contracts

### 1. Scope / Trigger

- Trigger: `sql-lens-proxy` owns the second TCP leg from an accepted client connection to the configured backend database address.
- Backend dialing is a proxy runtime boundary. It must preserve client/backend context for later forwarding and lifecycle recording.
- This layer must not forward bytes, parse SQL protocols, emit capture events, allocate connection IDs, or start the application runtime.

### 2. Signatures

Public backend dialing types live in `crates/sql-lens-proxy/src/lib.rs`:

```rust
pub struct BackendDialConfig {
    pub address: String,
    pub connect_timeout: std::time::Duration,
}

impl BackendDialConfig {
    pub fn new(address: impl Into<String>, connect_timeout: std::time::Duration) -> Self;
    pub fn from_config(
        proxy: &sql_lens_config::ProxyConfig,
        backend: &sql_lens_config::BackendConfig,
    ) -> Self;
}

pub struct BackendDialer;

impl BackendDialer {
    pub async fn dial(
        accepted: AcceptedClient,
        config: &BackendDialConfig,
    ) -> Result<ProxiedConnection, BackendDialError>;
}
```

Allowed backend dialing dependencies:

```toml
sql-lens-config = { path = "../sql-lens-config" }
tokio = { version = "1", features = ["net", "sync", "time", "rt", "macros"] }
tracing = "0.1"
```

Do not add `thiserror`, `anyhow`, `tokio-util`, protocol crates, app crates, storage crates, database clients, retry libraries, or TLS libraries for this layer.

### 3. Contracts

- `BackendDialConfig.address` is copied from `BackendConfig.address`.
- `BackendDialConfig.connect_timeout` is `Duration::from_millis(ProxyConfig.connect_timeout_ms)`.
- `BackendDialer::dial` consumes an `AcceptedClient`.
- Successful dial returns `ProxiedConnection` with the client stream, backend stream, client peer address, and backend address string.
- Failed dial drops the accepted client stream by ownership and returns `BackendDialError`.
- Timeout wraps the whole `TcpStream::connect` future with `tokio::time::timeout`.
- Connect failures preserve the source `std::io::Error`.
- Dial failure records are lightweight proxy-local records, not durable lifecycle records.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| Backend dial succeeds | Return `ProxiedConnection` |
| Backend dial future exceeds `connect_timeout` | Return `BackendDialError::Timeout { failure }` |
| Backend TCP connect returns an IO error | Return `BackendDialError::Connect { failure, source }` |
| Timeout failure is returned | `failure.kind` is `BackendDialFailureKind::Timeout { timeout }` |
| Connect failure is returned | `failure.kind` is `BackendDialFailureKind::Connect` |
| Runtime config is converted | Use `BackendConfig.address` and `ProxyConfig.connect_timeout_ms` only |
| Later forwarding is needed | Add it in the forwarding task using `ProxiedConnection`; do not extend dialing to copy bytes |

### 5. Good/Base/Bad Cases

Good:

- Dial a backend listener bound to `127.0.0.1:0` in tests and assert `ProxiedConnection` preserves addresses.
- Use a private pending future helper in tests to make timeout behavior deterministic without relying on OS TCP timing.
- Keep low-sensitivity logs to lifecycle addresses and timeout durations.

Base:

- A refused loopback port returns a structured connect failure.
- Future lifecycle work maps `BackendDialFailure` into a durable connection record.

Bad:

- Calling `tokio::io::copy_bidirectional` inside backend dialing.
- Importing protocol adapters to decide how to dial TCP.
- Creating connection IDs or writing storage records in `sql-lens-proxy`.
- Retrying backend dials without a dedicated retry-policy task.

### 6. Tests Required

For backend dialing changes:

- Config conversion test for backend address and connect timeout.
- Successful dial test using a local backend listener.
- Structured connect failure test using an unused loopback port.
- Deterministic timeout test; prefer a pending connect future over OS-specific unreachable addresses.
- Run socket-binding tests outside sandboxes that deny local TCP binds.
- Run `cargo fmt --check`.
- Run `cargo check --workspace`.
- Run `cargo test --workspace`.
- Run `cargo clippy --workspace --all-targets -- -D warnings`.

### 7. Wrong vs Correct

#### Wrong

```rust
let mut client = accepted.into_stream();
let mut backend = TcpStream::connect(backend_addr).await?;
tokio::io::copy_bidirectional(&mut client, &mut backend).await?;
```

#### Correct

```rust
let dial_config = BackendDialConfig::from_config(&config.proxy, &config.backend);
let proxied = BackendDialer::dial(accepted, &dial_config).await?;
```

## Scenario: Bidirectional TCP Forwarding Contracts

### 1. Scope / Trigger

- Trigger: `sql-lens-proxy` owns raw byte forwarding between paired client and backend TCP streams.
- Forwarding is the hot path for transparent proxy behavior. It must move bytes and report counters without waiting for storage, UI, protocol parsing, plugins, or exporters.
- This layer must not parse SQL protocols, emit capture events, allocate connection IDs, persist lifecycle records, or start the application runtime.

### 2. Signatures

Public forwarding types live in `crates/sql-lens-proxy/src/lib.rs`:

```rust
pub struct TcpForwarder;

impl TcpForwarder {
    pub async fn forward(
        connection: ProxiedConnection,
    ) -> Result<ForwardingSummary, ForwardingError>;
}

pub struct ForwardingSummary {
    pub client_peer_addr: std::net::SocketAddr,
    pub backend_address: String,
    pub client_to_backend_bytes: u64,
    pub backend_to_client_bytes: u64,
}

pub struct ForwardingFailure {
    pub client_peer_addr: std::net::SocketAddr,
    pub backend_address: String,
    pub client_to_backend_bytes: Option<u64>,
    pub backend_to_client_bytes: Option<u64>,
}

pub enum ForwardingError {
    Io {
        failure: ForwardingFailure,
        source: std::io::Error,
    },
}
```

Allowed forwarding dependencies:

```toml
tokio = { version = "1", features = ["net", "sync", "time", "rt", "macros", "io-util"] }
tracing = "0.1"
```

Do not add `tokio-util`, `thiserror`, `anyhow`, protocol crates, app crates, storage crates, database clients, retry libraries, or TLS libraries for this layer.

### 3. Contracts

- `TcpForwarder::forward` consumes a `ProxiedConnection`.
- Client stream must be passed as the first argument to `tokio::io::copy_bidirectional`.
- Backend stream must be passed as the second argument to `tokio::io::copy_bidirectional`.
- Successful forwarding returns `ForwardingSummary`.
- `client_to_backend_bytes` maps to the first `copy_bidirectional` return value.
- `backend_to_client_bytes` maps to the second `copy_bidirectional` return value.
- A clean EOF from either side relies on Tokio's close behavior: shutdown the opposite writer and finish when both directions close.
- IO failures preserve the source `std::io::Error` and proxy-local connection context.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| Client writes bytes | Backend receives the same bytes |
| Backend writes bytes | Client receives the same bytes |
| `copy_bidirectional` returns `Ok((a_to_b, b_to_a))` | Return `ForwardingSummary` with `a_to_b` as client-to-backend and `b_to_a` as backend-to-client |
| One side cleanly shuts down | Forwarding completes cleanly after both directions close |
| `copy_bidirectional` returns an IO error | Return `ForwardingError::Io { failure, source }` |
| Later protocol capture is needed | Add a protocol-aware observation layer in a later task; do not parse in `TcpForwarder` |

### 5. Good/Base/Bad Cases

Good:

- Use real loopback TCP streams in tests and assert exact bytes on the opposite side.
- Test byte counts in both directions so tuple order cannot silently flip.
- Use `tokio::time::timeout` around forwarding tests to catch hangs.

Base:

- Future session orchestration calls listener -> backend dialer -> forwarder.
- Future lifecycle work maps `ForwardingSummary` and `ForwardingFailure` into durable records.

Bad:

- Hand-rolling two copy loops before Tokio's behavior proves insufficient.
- Blocking forwarding on storage, WebSocket, plugin hooks, or metrics exporters.
- Logging raw SQL text or packet payloads from forwarding code.
- Adding protocol-specific conditions to forwarding.

### 6. Tests Required

For TCP forwarding changes:

- Client-to-backend copy test.
- Backend-to-client copy test.
- Bidirectional byte counter test.
- Clean close completion test.
- Run socket-binding tests outside sandboxes that deny local TCP binds.
- Run `cargo fmt --check`.
- Run `cargo check --workspace`.
- Run `cargo test --workspace`.
- Run `cargo clippy --workspace --all-targets -- -D warnings`.

### 7. Wrong vs Correct

#### Wrong

```rust
let sql = parse_mysql_packet(&buffer)?;
storage.insert(sql).await?;
backend.write_all(&buffer).await?;
```

#### Correct

```rust
let summary = TcpForwarder::forward(proxied).await?;
```

## Scenario: Proxy Connection Lifecycle Contracts

### 1. Scope / Trigger

- Trigger: `sql-lens-proxy` records proxy-local connection lifecycle state for accepted client sessions.
- Lifecycle tracking bridges listener, backend dialing, and forwarding summaries.
- This layer must remain protocol-neutral and in-memory until storage/API/runtime orchestration tasks explicitly consume it.

### 2. Signatures

Public lifecycle types live in `crates/sql-lens-proxy/src/lib.rs`:

```rust
pub struct ConnectionLifecycleIdGenerator;

impl ConnectionLifecycleIdGenerator {
    pub fn new() -> Self;
    pub fn next_id(&self) -> sql_lens_core::ConnectionId;
}

pub struct ConnectionLifecycleRecord;

impl ConnectionLifecycleRecord {
    pub fn accepted(
        id: sql_lens_core::ConnectionId,
        protocol: sql_lens_core::ProtocolName,
        database_type: sql_lens_core::DatabaseType,
        client_addr: impl Into<String>,
        backend_addr: impl Into<String>,
        accepted_at: sql_lens_core::Timestamp,
    ) -> Self;

    pub fn info(&self) -> &sql_lens_core::ConnectionInfo;
    pub fn transitions(&self) -> &[ConnectionLifecycleTransition];
    pub fn failure(&self) -> Option<&ConnectionLifecycleFailure>;
    pub fn into_info(self) -> sql_lens_core::ConnectionInfo;
    pub fn mark_backend_connected(&mut self, connected_at: sql_lens_core::Timestamp);
    pub fn mark_forwarding_closed(&mut self, summary: &ForwardingSummary, closed_at: sql_lens_core::Timestamp);
    pub fn mark_backend_dial_failed(&mut self, failure: &BackendDialFailure, failed_at: sql_lens_core::Timestamp);
    pub fn mark_forwarding_failed(&mut self, failure: &ForwardingFailure, failed_at: sql_lens_core::Timestamp);
}
```

Allowed lifecycle dependency addition:

```toml
sql-lens-core = { path = "../sql-lens-core" }
```

Do not add UUID, time/chrono, storage, API, app runtime, protocol adapter, capture pipeline, or database client dependencies for this layer.

### 3. Contracts

- `ConnectionLifecycleIdGenerator` produces stable process-local `ConnectionId` values.
- `ConnectionLifecycleRecord::accepted` creates a `ConnectionInfo` with `ConnectionState::Created`, client address, backend address, protocol, database type, zero byte counters, zero query count, and no user/database.
- State transitions are recorded in `ConnectionLifecycleTransition` so intermediate states such as `Closing` are observable even when final state is `Closed`.
- `mark_backend_connected` transitions to `ConnectionState::BackendConnected`.
- `mark_forwarding_closed` updates `bytes_in` from `ForwardingSummary.client_to_backend_bytes`, updates `bytes_out` from `ForwardingSummary.backend_to_client_bytes`, records `Closing`, then records `Closed`.
- `mark_backend_dial_failed` maps `BackendDialFailureKind` into `ConnectionLifecycleFailureKind` and transitions to `Failed`.
- `mark_forwarding_failed` preserves any available byte counters from `ForwardingFailure`, records forwarding failure context, and transitions to `Failed`.
- Timestamps are supplied by callers as core-owned `Timestamp` values; this layer does not create wall-clock timestamps.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| Accepted client creates a record | State is `Created`, byte counters are zero, transition history contains `Created` |
| Backend dial succeeds | State becomes `BackendConnected` and transition history records it |
| Forwarding completes cleanly | Byte counters are copied, `closed_at` is set, transition history records `Closing` then `Closed` |
| Backend dial times out | State becomes `Failed`, `closed_at` is set, failure kind is `BackendDialTimeout` |
| Backend dial returns connect error | State becomes `Failed`, `closed_at` is set, failure kind is `BackendDialConnect` |
| Forwarding fails with byte counters | Known counters are copied before state becomes `Failed` |
| Future storage/API exposure is needed | Consume `ConnectionInfo` and transition data from a later task; do not add storage/API here |

### 5. Good/Base/Bad Cases

Good:

- A session orchestrator later creates one lifecycle record immediately after accept and updates it after backend dial and forwarding.
- Protocol-specific data stays outside lifecycle records unless represented as protocol-neutral core metadata in a later design.

Base:

- Tests construct synthetic `ForwardingSummary` and `BackendDialFailure` values and assert state transitions without opening sockets.
- The lifecycle record can be converted into `ConnectionInfo` for future storage or API layers.

Bad:

- Generating UUIDs or wall-clock timestamps inside `sql-lens-proxy` lifecycle code.
- Writing lifecycle records to SQLite from proxy hot-path primitives.
- Adding MySQL-only fields to `ConnectionLifecycleRecord` or `ConnectionInfo`.
- Blocking forwarding on lifecycle persistence, exporters, WebSocket, or plugin hooks.

### 6. Tests Required

For proxy lifecycle changes:

- ID generation test for deterministic process-local IDs.
- Accepted record construction test or normal-close test that asserts initial `Created` state.
- Backend-connected transition assertion.
- Normal forwarding close test that asserts byte counters, `Closing`, and `Closed`.
- Backend dial failure test that asserts `Failed`, `closed_at`, and failure-kind mapping.
- Forwarding failure test when byte-counter behavior changes.
- Run `cargo fmt --check`.
- Run `cargo check --workspace`.
- Run `cargo test --workspace`.
- Run `cargo clippy --workspace --all-targets -- -D warnings`.

### 7. Wrong vs Correct

#### Wrong

```rust
let id = uuid::Uuid::new_v4();
sqlite.insert_connection(connection).await?;
```

#### Correct

```rust
let id = lifecycle_ids.next_id();
let mut lifecycle = ConnectionLifecycleRecord::accepted(
    id,
    ProtocolName("mysql".to_owned()),
    DatabaseType("mysql".to_owned()),
    client_addr,
    backend_addr,
    accepted_at,
);
lifecycle.mark_backend_connected(backend_connected_at);
```

## Scenario: Application Connection Lifecycle Runtime Wiring

### 1. Scope / Trigger

- Trigger: `sql-lens-app` composes accepted MySQL proxy sessions with the
  existing lifecycle, connection-store, and live-statistics contracts.
- The app runtime owns this fan-out; `sql-lens-proxy` stays protocol-neutral
  and must not depend on storage or API crates.

### 2. Signatures

Runtime composition uses the existing types and private app helpers:

```rust
async fn record_connection_started(
    state: &sql_lens_api::ApiState,
    lifecycle: &sql_lens_proxy::ConnectionLifecycleRecord,
);

async fn finalize_forwarding_lifecycle(
    state: &sql_lens_api::ApiState,
    lifecycle: &mut sql_lens_proxy::ConnectionLifecycleRecord,
    forwarding_result: &Result<
        sql_lens_proxy::ForwardingSummary,
        sql_lens_proxy::ForwardingError,
    >,
);
```

### 3. Contracts

- Create one `ConnectionLifecycleRecord` immediately after accepting a client,
  before backend dialing, using the configured target identity and backend
  address.
- After a successful backend dial, mark the record `backend_connected`, upsert
  it in `ConnectionStore`, and call `LiveStatistics::record_connection_opened`
  before starting forwarding.
- A backend dial failure becomes a retained terminal `failed` connection
  record. It must not be recorded as active.
- On either forwarding outcome, update the same lifecycle record first, then
  upsert the final `ConnectionInfo` and call
  `LiveStatistics::record_connection_closed`. Normal completion is `closed`;
  I/O failure is `failed` and retains available byte counters.
- Connection history remains in-memory and bounded by `ConnectionStore`.
  Do not persist it to SQLite or change REST schemas in runtime wiring work.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| Backend dial succeeds | Store `backend_connected`; increment active connections once |
| Forwarding closes normally | Store `closed` with final byte counts and `closed_at`; decrement active connections |
| Forwarding returns I/O error | Store `failed` with available byte counts and `closed_at`; decrement active connections |
| Backend dial fails | Store `failed` with `closed_at`; active connections stay unchanged |
| Closing an already non-active connection | Rely on idempotent statistics close behavior; do not remove its history record |

### 5. Good/Base/Bad Cases

Good:

- App runtime creates a lifecycle record before dialing, so an unreachable
  backend is visible through `GET /api/v1/connections`.
- A completed session replaces its active record under the same connection ID.

Base:

- Runtime uses `ApiState` accessors to obtain existing lock-protected stores;
  it does not add locks inside `sql-lens-storage`.

Bad:

- Recording active connections from SQL events instead of connection lifecycle.
- Dropping backend dial failures because no forwarding task was started.
- Writing storage or statistics state from `sql-lens-proxy`.

### 6. Tests Required

- Unit tests for active-to-closed and active-to-failed lifecycle finalization,
  including byte counters and active-connection count.
- Unit test that retains a dial failure without increasing active connections.
- Loopback runtime tests that assert the connections API exposes a completed
  proxied session and a failed backend dial.

### 7. Wrong vs Correct

#### Wrong

```rust
let connection = BackendDialer::dial(accepted, &config).await?;
tokio::spawn(forward_mysql_connection(connection));
```

#### Correct

```rust
let mut lifecycle = ConnectionLifecycleRecord::accepted(/* target context */);
match BackendDialer::dial(accepted, &config).await {
    Ok(connection) => {
        lifecycle.mark_backend_connected(runtime_timestamp());
        record_connection_started(&state, &lifecycle).await;
        // Forwarding finalizes and retains the same lifecycle record.
    }
    Err(error) => {
        lifecycle.mark_backend_dial_failed(error.failure(), runtime_timestamp());
        record_connection_finished(&state, lifecycle.into_info()).await;
    }
}
```

## Scenario: Capture Pipeline Channel Contracts

### 1. Scope / Trigger

- Trigger: `sql-lens-capture` owns the bounded handoff channel for normalized `SqlEvent` values.
- The capture channel sits between proxy/protocol capture producers and future storage/WebSocket/statistics consumers.
- This layer must keep packet forwarding non-blocking and protocol-neutral.

### 2. Signatures

Public capture types live in `crates/sql-lens-capture/src/lib.rs`:

```rust
pub struct CapturePipelineConfig {
    pub capacity: std::num::NonZeroUsize,
    pub overload_policy: CaptureOverloadPolicy,
}

pub enum CaptureOverloadPolicy {
    DropNewest,
    RejectNew,
}

pub struct CapturePipeline;

impl CapturePipeline {
    pub fn channel(config: CapturePipelineConfig) -> (CaptureEventPublisher, CaptureEventReceiver);
}

impl CaptureEventPublisher {
    pub fn publish(&self, event: sql_lens_core::SqlEvent) -> Result<CapturePublishOutcome, CapturePublishError>;
    pub fn stats(&self) -> CapturePipelineStats;
}

impl CaptureEventReceiver {
    pub async fn recv(&mut self) -> Option<sql_lens_core::SqlEvent>;
    pub fn stats(&self) -> CapturePipelineStats;
}
```

Allowed dependencies:

```toml
sql-lens-core = { path = "../sql-lens-core" }
tokio = { version = "1", features = ["sync"] }
```

Do not add proxy, protocol, storage, API, plugin, app, database client, HTTP, exporter, `tokio-util`, `thiserror`, `anyhow`, UUID, or time/chrono dependencies for this layer.

### 3. Contracts

- `CapturePipelineConfig.capacity` uses `NonZeroUsize`; zero-capacity channels are unrepresentable.
- `CapturePipeline::channel` returns one cloneable publisher and one receiver.
- `CaptureEventPublisher::publish` must use `tokio::sync::mpsc::Sender::try_send`; it must not await.
- `CaptureOverloadPolicy::DropNewest` drops the incoming event when the channel is full, increments `dropped_events`, and returns `CapturePublishOutcome::Dropped`.
- `CaptureOverloadPolicy::RejectNew` returns `CapturePublishError::Full { event }`, increments `dropped_events`, and leaves the queued event unchanged.
- Closed receivers return `CapturePublishError::Closed { event }` and do not increment the overload dropped counter.
- `CapturePipelineStats.dropped_events` is shared between publisher and receiver handles.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| Channel has capacity | `publish` returns `Enqueued` and receiver can read the same `SqlEvent` |
| Channel is full and policy is `DropNewest` | Incoming event is dropped, return `Dropped`, increment `dropped_events` |
| Channel is full and policy is `RejectNew` | Return `Full { event }`, increment `dropped_events`, keep queued event |
| Receiver is dropped | Return `Closed { event }`, do not increment `dropped_events` |
| Future storage fan-out is needed | Add a consumer task later; do not write storage inside publisher |

### 5. Good/Base/Bad Cases

Good:

- Protocol observers emit already-normalized `SqlEvent` values into `CaptureEventPublisher`.
- A later fan-out task owns the receiver and dispatches to storage, WebSocket, and counters.

Base:

- Tests use synthetic `SqlEvent` values from `sql-lens-core`.
- Backpressure behavior is verified by creating a capacity-one channel and publishing two events.

Bad:

- Calling `send().await` from the packet-forwarding path.
- Adding storage writes or WebSocket broadcast loops to `sql-lens-capture`.
- Dropping events without incrementing `dropped_events`.
- Using a zero-capacity mpsc channel and letting Tokio panic.

### 6. Tests Required

For capture pipeline changes:

- Enqueue/receive test that asserts the exact `SqlEvent` survives the channel.
- Drop-newest overload test that asserts only the first event is received and dropped count increments.
- Reject-new overload test that asserts the rejected event is returned and dropped count increments.
- Closed receiver test that asserts structured closed error and no overload count change.
- Run `cargo fmt --check`.
- Run `cargo check --workspace`.
- Run `cargo test --workspace`.
- Run `cargo clippy --workspace --all-targets -- -D warnings`.

### 7. Wrong vs Correct

#### Wrong

```rust
storage.append(event).await?;
publisher.send(event).await?;
```

#### Correct

```rust
match publisher.publish(event)? {
    CapturePublishOutcome::Enqueued => {}
    CapturePublishOutcome::Dropped => {}
}
```

## Scenario: Slow SQL Classification Contracts

### 1. Scope / Trigger

- Trigger: `sql-lens-capture` classifies normalized `SqlEvent` values as slow
  before storage, WebSocket broadcast, live statistics, or API exposure.
- Classification is protocol-neutral capture enrichment. Protocol adapters
  still emit `ok` or `error` from backend terminal packets.
- This layer must not parse SQL text, inspect protocol metadata, write storage,
  call APIs, or block packet forwarding.

### 2. Signatures

Public slow classification types live in `crates/sql-lens-capture/src/lib.rs`:

```rust
pub const DEFAULT_SLOW_THRESHOLD_MS: u64 = 500;

pub struct SlowQueryClassifier;

impl SlowQueryClassifier {
    pub fn new(threshold: sql_lens_core::DurationMillis) -> Self;
    pub fn threshold(&self) -> sql_lens_core::DurationMillis;
    pub fn classify(&self, event: sql_lens_core::SqlEvent) -> sql_lens_core::SqlEvent;
}
```

Config exposes the global threshold under `[proxy]` while runtime composition is
still proxy-first:

```toml
[proxy]
slow_threshold_ms = 500
```

### 3. Contracts

- Classification consumes and returns a full `SqlEvent`; it does not mutate
  shared storage in place.
- `CaptureStatus::Ok` becomes `CaptureStatus::Slow` when
  `event.duration >= threshold`.
- `CaptureStatus::Ok` remains `Ok` below the threshold.
- `CaptureStatus::Error`, `Unknown`, and already-`Slow` events are unchanged.
- Threshold `0` is valid and classifies every successful event as slow.
- App fan-out must classify once before cloning the event to storage,
  WebSocket broadcast, and live statistics.
- Storage, API handlers, WebSocket subscriptions, and statistics counters must
  consume the classified status instead of reimplementing threshold checks.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| OK event duration is below threshold | Status remains `Ok` |
| OK event duration equals threshold | Status becomes `Slow` |
| OK event duration exceeds threshold | Status becomes `Slow` |
| Error event exceeds threshold | Status remains `Error` |
| Unknown event exceeds threshold | Status remains `Unknown` |
| Already slow event is classified again | Status remains `Slow` |
| Classified event enters app fan-out | Stored event and live statistics see the classified status |

### 5. Good/Base/Bad Cases

Good:

- `sql-lens-app` classifies once in capture fan-out and then records the same
  classified event in the ring buffer and live statistics.

Base:

- Unit tests use synthetic `SqlEvent` values with deterministic durations.

Bad:

- MySQL adapter deciding that a successful OK packet should emit `Slow`.
- API code recalculating slow status from `duration_ms`.
- Live statistics applying a private threshold instead of counting
  `CaptureStatus::Slow`.

### 6. Tests Required

- Classifier unit tests for below, equal, and above threshold.
- Classifier unit tests for error, unknown, and already-slow statuses.
- Config default and TOML parsing tests for `slow_threshold_ms`.
- App fan-out test proving storage and live statistics receive classified
  events.
- Run `cargo fmt --check`.
- Run `cargo test --workspace`.
- Run `cargo clippy --workspace --all-targets -- -D warnings`.

### 7. Wrong vs Correct

#### Wrong

```rust
if response.status == "ok" && response.duration_ms > 500 {
    response.status = "slow".to_owned();
}
```

#### Correct

```rust
let event = classifier.classify(event);
store.append(event.clone());
statistics.record_sql_event(&event);
```

## Scenario: Ring Buffer Storage Contracts

### 1. Scope / Trigger

- Trigger: `sql-lens-storage` owns the default in-memory storage backend for retained SQL events.
- Ring buffer append and timeline query are the first storage primitives and must stay append-oriented.
- This layer bounds memory by capacity and evicts oldest events first.

### 2. Signatures

Public ring buffer types live in `crates/sql-lens-storage/src/lib.rs`:

```rust
pub struct RingBufferStore;

impl RingBufferStore {
    pub fn new(capacity: std::num::NonZeroUsize) -> Self;
    pub fn append(&mut self, event: sql_lens_core::SqlEvent) -> RingBufferAppendOutcome;
    pub fn get(&self, id: &sql_lens_core::SqlEventId) -> Option<&sql_lens_core::SqlEvent>;
    pub fn query_timeline(
        &self,
        query: RingBufferTimelineQuery,
    ) -> Result<RingBufferTimelinePage, SqlEventFilterError>;
    pub fn snapshot(&self) -> Vec<sql_lens_core::SqlEvent>;
    pub fn stats(&self) -> RingBufferStats;
    pub fn len(&self) -> usize;
    pub fn capacity(&self) -> usize;
    pub fn is_empty(&self) -> bool;
}

pub struct RingBufferAppendOutcome {
    pub stored_event_id: sql_lens_core::SqlEventId,
    pub evicted_event_id: Option<sql_lens_core::SqlEventId>,
}

pub struct RingBufferTimelineQuery {
    pub limit: std::num::NonZeroUsize,
    pub cursor: Option<RingBufferTimelineCursor>,
    pub filter: SqlEventFilter,
}

pub struct RingBufferTimelineCursor {
    pub before_sequence: u64,
}

pub struct RingBufferTimelinePage {
    pub events: Vec<sql_lens_core::SqlEvent>,
    pub next_cursor: Option<RingBufferTimelineCursor>,
}

pub struct SqlEventFilter {
    pub protocol: Option<sql_lens_core::ProtocolName>,
    pub database_type: Option<sql_lens_core::DatabaseType>,
    pub database: Option<String>,
    pub user: Option<String>,
    pub client_addr: Option<String>,
    pub status: Option<sql_lens_core::CaptureStatus>,
    pub min_duration: Option<sql_lens_core::DurationMillis>,
    pub max_duration: Option<sql_lens_core::DurationMillis>,
    pub text: Option<String>,
    pub fingerprint: Option<String>,
    pub from: Option<sql_lens_core::Timestamp>,
    pub to: Option<sql_lens_core::Timestamp>,
}

pub enum SqlEventFilterError {
    InvalidDurationRange {
        min: sql_lens_core::DurationMillis,
        max: sql_lens_core::DurationMillis,
    },
    InvalidTimestampRange {
        from: sql_lens_core::Timestamp,
        to: sql_lens_core::Timestamp,
    },
}

pub struct RingBufferStats {
    pub capacity: usize,
    pub len: usize,
    pub total_appended: u64,
    pub total_evicted: u64,
}
```

Allowed dependency:

```toml
sql-lens-core = { path = "../sql-lens-core" }
```

Do not add async runtime, database, API, protocol, app, HTTP, serialization, or concurrency dependencies for the basic ring buffer append layer.

### 3. Contracts

- Capacity is represented by `NonZeroUsize`; zero-capacity stores are unrepresentable.
- Events are stored in insertion order.
- `append` stores the incoming `SqlEvent` without mutating it.
- If the buffer is full, `append` evicts exactly one oldest event with `pop_front` semantics.
- `RingBufferAppendOutcome.stored_event_id` is the incoming event ID.
- `RingBufferAppendOutcome.evicted_event_id` is the evicted oldest event ID, if any.
- `RingBufferStats.total_appended` increments once per append.
- `RingBufferStats.total_evicted` increments once per evicted event.
- `snapshot` returns retained events in oldest-to-newest order.
- `get` returns a borrowed retained event by ID.
- `get` returns `None` for evicted or missing events.
- `get` must not mutate store state or stats.
- Each appended event receives an internal monotonically increasing sequence.
- `query_timeline` returns cloned retained events in newest-to-oldest order.
- `RingBufferTimelineQuery.limit` is non-zero and bounds the returned page size.
- A missing timeline cursor means query from the newest retained event.
- `RingBufferTimelineCursor.before_sequence = N` means return retained events with sequence `< N`.
- `RingBufferTimelinePage.next_cursor` is returned only when older retained events are available.
- `next_cursor.before_sequence` points to the oldest returned event sequence.
- A cursor remains stable across newer appends because newer events have larger sequences and are excluded from older-page queries.
- Evicted events may naturally disappear from later cursor pages.
- `SqlEventFilter::default()` means no filtering.
- Supported storage filters are strongly typed: protocol, database type, database, user, client address, status, minimum duration, maximum duration, SQL text, fingerprint, start timestamp, and end timestamp.
- Storage combines filter fields with logical AND.
- Client address filtering matches `SqlEvent.client_addr` exactly.
- Fingerprint filtering matches `SqlEvent.fingerprint` exactly when present.
- SQL text filtering performs a case-sensitive substring match against `original_sql`, `normalized_sql`, and `expanded_sql`; storage must not parse SQL.
- Time range filtering uses the current `Timestamp` string ordering and assumes sortable captured timestamp strings; storage must not add timestamp parsing in this layer.
- `min_duration > max_duration` returns `SqlEventFilterError::InvalidDurationRange`.
- `from > to` returns `SqlEventFilterError::InvalidTimestampRange`.
- Filtered cursor pagination returns `next_cursor` only when an older retained event matching the same filter exists.
- Unknown HTTP query parameters belong to the API layer and must not be modeled as loose string filters in storage.
- Retention, persistent cursor serialization, SQLite/DuckDB filters, API query parsing, WebSocket filters, and secondary indexes belong to later tasks.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| Store is empty and append occurs | Store length becomes 1, no eviction is reported |
| Store has room and append occurs | Store length grows by 1, no eviction is reported |
| Store is full and append occurs | Oldest event is evicted, incoming event is appended |
| Capacity is 1 and two events append | Only second event remains |
| Capacity is zero | Cannot construct through `NonZeroUsize` |
| Retained event is looked up by ID | Return `Some(&SqlEvent)` |
| Evicted event is looked up by ID | Return `None` |
| Missing event is looked up by ID | Return `None` |
| Snapshot is requested | Return cloned retained events in insertion order |
| Timeline query has no cursor | Return newest retained events first, up to the non-zero limit |
| Timeline query limit is smaller than retained events | Return `next_cursor` for the next older page |
| Timeline query uses `before_sequence = N` | Return only retained events with internal sequence `< N` |
| New events append after a cursor is issued | The cursor still returns the older page, excluding newer events |
| Older cursor-targeted events were evicted | Return the remaining retained older events without error |
| Timeline query has `SqlEventFilter::default()` | Behave like the unfiltered timeline query |
| Multiple filter fields are set | Return only events matching all fields |
| Client address filter is set | Match `SqlEvent.client_addr` exactly |
| Fingerprint filter is set | Match `SqlEvent.fingerprint` exactly |
| Text filter is set | Match stored SQL text fields by case-sensitive substring |
| Time range filter is set | Compare `Timestamp` string values without parsing |
| `min_duration > max_duration` | Return `SqlEventFilterError::InvalidDurationRange` before scanning |
| `from > to` | Return `SqlEventFilterError::InvalidTimestampRange` before scanning |
| Filtered query has older retained non-matching events only | Return no `next_cursor` |

### 5. Good/Base/Bad Cases

Good:

- Ring buffer append stays synchronous and in-memory.
- Timeline query scans retained entries directly; no index is needed until filters or scale require it.
- Storage filters stay strongly typed and protocol-neutral.
- Tests use synthetic `SqlEvent` values from `sql-lens-core`.

Base:

- Future performance work may add an ID index while preserving append and eviction semantics.
- Future retention work may add age/byte eviction after this oldest-first baseline.
- Future API work may map query parameters into `SqlEventFilter` and reject unknown parameters before storage.

Bad:

- Adding secondary indexes before query behavior proves they are needed.
- Exposing internal sequence on `SqlEvent`.
- Treating ring buffer timeline cursors as durable API tokens before API pagination is designed.
- Accepting arbitrary string filter fields inside storage to mimic HTTP query parsing.
- Parsing SQL or timestamps inside the ring buffer filter layer.
- Blocking append on SQLite, API, WebSocket, or async runtime work.
- Allowing capacity zero and relying on runtime panics or special cases.
- Mutating `SqlEvent` during storage append.

### 6. Tests Required

For ring buffer append changes:

- Append test for an empty store.
- Capacity enforcement test.
- Oldest eviction test.
- Stats test for appended and evicted counters.
- Existing event lookup test.
- Evicted event lookup test.
- Non-zero capacity test.
- Timeline newest-first ordering test.
- Timeline limit and next-cursor test.
- Timeline cursor paging test with no duplicate events across pages.
- Timeline cursor stability test after newer append.
- At least five combined filter tests covering protocol, database type, database, user, client address, status, duration, SQL text, fingerprint, and time range behavior.
- Filtered cursor pagination test proving `next_cursor` is based on older matching events.
- Invalid duration range test.
- Invalid timestamp range test.
- Run `cargo fmt --check`.
- Run `cargo check --workspace`.
- Run `cargo test --workspace`.
- Run `cargo clippy --workspace --all-targets -- -D warnings`.

### 7. Wrong vs Correct

#### Wrong

```rust
pub fn append(&mut self, event: SqlEvent) {
    self.events.push(event);
}
```

#### Correct

```rust
if self.events.len() == self.capacity.get() {
    self.events.pop_front();
}
self.events.push_back(event);
```

## Scenario: Connection Store Contracts

### 1. Scope / Trigger

- Trigger: `sql-lens-storage` owns the in-memory connection state store used by the connections API.
- The store keeps the latest retained `ConnectionInfo` per connection ID.
- This layer must not start proxy runtime work, install async locks, persist to SQLite/DuckDB, or format API DTOs.

### 2. Signatures

Public connection store types live in `crates/sql-lens-storage/src/lib.rs`:

```rust
pub struct ConnectionStore;

impl ConnectionStore {
    pub fn new(capacity: std::num::NonZeroUsize) -> Self;
    pub fn upsert(&mut self, connection: sql_lens_core::ConnectionInfo) -> ConnectionUpsertOutcome;
    pub fn list_recent(&self, limit: std::num::NonZeroUsize) -> Vec<sql_lens_core::ConnectionInfo>;
    pub fn get(&self, id: &sql_lens_core::ConnectionId) -> Option<&sql_lens_core::ConnectionInfo>;
    pub fn len(&self) -> usize;
    pub fn capacity(&self) -> usize;
    pub fn is_empty(&self) -> bool;
}

pub struct ConnectionUpsertOutcome {
    pub stored_connection_id: sql_lens_core::ConnectionId,
    pub replaced_existing: bool,
    pub evicted_connection_id: Option<sql_lens_core::ConnectionId>,
}
```

Allowed dependency remains:

```toml
sql-lens-core = { path = "../sql-lens-core" }
```

### 3. Contracts

- Capacity is represented by `NonZeroUsize`; zero-capacity stores are unrepresentable.
- `upsert` stores the latest `ConnectionInfo` for a `ConnectionId`.
- If an upsert ID already exists, replace it and move it to the newest position.
- If an upsert ID is new and the store is full, evict the oldest-updated connection.
- `ConnectionUpsertOutcome.stored_connection_id` is the incoming connection ID.
- `ConnectionUpsertOutcome.replaced_existing` is `true` only when an existing ID was replaced.
- `ConnectionUpsertOutcome.evicted_connection_id` is set only when a different oldest connection was evicted.
- `list_recent` returns cloned connections newest-first.
- `get` returns a borrowed retained connection by ID.
- `get` returns `None` for missing or evicted connections.
- Connection store ordering is update-recency, not original connection time.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| Empty store receives a connection | Store length becomes 1 |
| Existing ID is upserted | Replace old value, length unchanged, move to newest |
| New ID is upserted when full | Evict oldest-updated connection |
| `list_recent` limit is smaller than stored count | Return newest `limit` items |
| Existing connection is looked up | Return `Some(&ConnectionInfo)` |
| Missing or evicted connection is looked up | Return `None` |

### 5. Good/Base/Bad Cases

Good:

- Proxy lifecycle can later upsert active and closed `ConnectionInfo` values into the same store.
- API tests inject a `ConnectionStore` through `ApiState::with_stores`.

Base:

- A closed connection replaces its earlier active connection record.

Bad:

- Adding `tokio::sync::RwLock` inside `sql-lens-storage`.
- Returning API-shaped JSON structs from storage.
- Treating the connection store as durable persistence before SQLite/DuckDB work exists.

### 6. Tests Required

For connection store changes:

- Upsert active connection test.
- Update existing connection to closed test.
- Recent list newest-first test.
- Existing lookup test.
- Missing and evicted lookup test.
- Capacity and empty-state test.
- Run `cargo fmt --check`.
- Run `cargo check --workspace`.
- Run `cargo test --workspace`.
- Run `cargo clippy --workspace --all-targets -- -D warnings`.

### 7. Wrong vs Correct

#### Wrong

```rust
pub struct ConnectionStore {
    inner: tokio::sync::RwLock<Vec<ConnectionResponse>>,
}
```

#### Correct

```rust
pub struct ConnectionStore {
    capacity: NonZeroUsize,
    connections: VecDeque<ConnectionInfo>,
}
```

## Scenario: Live Statistics Counter Contracts

### 1. Scope / Trigger

- Trigger: `sql-lens-storage` owns lightweight in-memory live statistics helpers for dashboard-ready metrics.
- Live statistics are incremental counters fed by retained or captured `SqlEvent` values.
- This layer is not a historical analytics engine and must not start API, WebSocket, async runtime, or persistence work.

### 2. Signatures

Public live statistics types live in `crates/sql-lens-storage/src/live_statistics.rs` and are re-exported from `crates/sql-lens-storage/src/lib.rs`:

```rust
pub struct LiveStatistics;

impl LiveStatistics {
    pub fn new() -> Self;
    pub fn record_sql_event(&mut self, event: &sql_lens_core::SqlEvent);
    pub fn record_sql_event_at(&mut self, event: &sql_lens_core::SqlEvent, recorded_at: std::time::Instant);
    pub fn record_connection_opened(&mut self, connection_id: sql_lens_core::ConnectionId);
    pub fn record_connection_closed(&mut self, connection_id: &sql_lens_core::ConnectionId);
    pub fn snapshot(&mut self) -> LiveStatisticsSnapshot;
    pub fn snapshot_at(&mut self, now: std::time::Instant) -> LiveStatisticsSnapshot;
}

pub struct LiveStatisticsSnapshot {
    pub total_events: u64,
    pub error_events: u64,
    pub slow_events: u64,
    pub qps_window_secs: u64,
    pub qps: f64,
    pub latency_buckets: Vec<LatencyBucketCount>,
    pub latency_percentiles: LatencyPercentiles,
    pub active_connections: usize,
}

pub struct LatencyPercentiles {
    pub p50: f64,
    pub p95: f64,
    pub p99: f64,
}

pub struct LatencyBucketCount {
    pub upper_bound: Option<sql_lens_core::DurationMillis>,
    pub count: u64,
}
```

Allowed dependency remains:

```toml
sql-lens-core = { path = "../sql-lens-core" }
```

Do not add async runtime, API, WebSocket, storage persistence, metrics library, `time`, `uuid`, or concurrency dependencies for this counter layer.

### 3. Contracts

- `LiveStatistics::default()` must be equivalent to `LiveStatistics::new()`.
- `record_sql_event` uses ingestion time from `Instant::now()`.
- `record_sql_event_at` exists for deterministic tests and controlled fan-out code.
- `total_events` increments for every SQL event.
- `error_events` increments only when `SqlEvent.status == CaptureStatus::Error`.
- `slow_events` increments only when `SqlEvent.status == CaptureStatus::Slow`.
- `CaptureStatus::Ok` and `CaptureStatus::Unknown` increment total and latency buckets only.
- QPS uses a fixed 60-second live window based on ingestion `Instant`, not `SqlEvent.timestamp`.
- QPS is `recent_events_in_window / 60.0`.
- Recent event timestamps and latency samples are pruned on record and snapshot calls.
- Latency buckets are fixed: `<=1ms`, `<=5ms`, `<=10ms`, `<=50ms`, `<=100ms`, `<=500ms`, `<=1000ms`, `<=5000ms`, and `>5000ms`.
- The overflow latency bucket uses `upper_bound = None`.
- `latency_percentiles` are exact p50/p95/p99 values over retained recent live latency samples in the fixed 60-second window.
- Empty percentile snapshots return `0.0` for p50, p95, and p99.
- Active connections are explicit lifecycle updates through open/close methods, not inferred from SQL events.
- Repeated opens for the same `ConnectionId` are idempotent.
- Closing a missing connection is a no-op.
- Top fingerprints, top users, top databases, persistent statistics, historical statistics queries, and WebSocket statistics streams belong to later tasks.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| OK event is recorded | Increment total and one latency bucket only |
| Slow event is recorded | Increment total, slow count, and one latency bucket |
| Error event is recorded | Increment total, error count, and one latency bucket |
| Unknown event is recorded | Increment total and one latency bucket only |
| Event duration is exactly a bucket upper bound | Count it in that upper-bound bucket |
| Event duration is greater than 5000ms | Count it in the overflow bucket |
| Snapshot is requested after more than 60 seconds | Prune older recent event timestamps before QPS calculation and older latency samples before percentile calculation |
| Same connection opens twice | Active connection count remains one for that ID |
| Missing connection closes | Active connection count is unchanged |
| Statistics API receives an unsupported `window` value | Return the standard `BAD_REQUEST` API envelope |

### 5. Good/Base/Bad Cases

Good:

- A future capture fan-out task calls `record_sql_event_at` once per accepted `SqlEvent`.
- A future proxy lifecycle fan-out calls connection open/close methods when sessions start and finish.
- API code reads a snapshot and serializes it without recalculating live counters.

Base:

- Tests use deterministic `Instant` values and synthetic `SqlEvent` values.
- Historical dashboard queries later use storage queries, not these live counters.

Bad:

- Inferring active connections from `SqlEvent.connection_id`.
- Parsing `SqlEvent.timestamp` for live QPS.
- Approximating percentiles from coarse latency bucket counters.
- Adding top-N ranking logic to this live counter helper.
- Blocking packet forwarding on live statistics updates.

### 6. Tests Required

For live statistics changes:

- OK, slow, and error event counter test.
- Latency bucket boundary and overflow test.
- Empty and populated latency percentile tests.
- Latency percentile window pruning test.
- Fixed 60-second QPS window test using deterministic `Instant` values.
- Active connection open/close idempotency test.
- Statistics API empty, populated, and invalid-window tests when changing the API endpoint.
- Run `cargo fmt --check`.
- Run `cargo check --workspace`.
- Run `cargo test --workspace`.
- Run `cargo clippy --workspace --all-targets -- -D warnings`.

### 7. Wrong vs Correct

#### Wrong

```rust
let active_connections = events
    .iter()
    .map(|event| event.connection_id.clone())
    .collect::<HashSet<_>>()
    .len();
```

#### Correct

```rust
statistics.record_connection_opened(connection_id.clone());
statistics.record_connection_closed(&connection_id);
```

## Scenario: Protocol Adapter Trait Contracts

### 1. Scope / Trigger

- Trigger: `sql-lens-protocol` defines shared contracts for protocol-specific adapters.
- The contract is consumed by future MySQL/PostgreSQL/ClickHouse adapters and by the adapter registry.
- This layer must stay protocol-neutral and object-safe.

### 2. Signatures

Public protocol adapter types live in `crates/sql-lens-protocol/src/lib.rs`:

```rust
pub trait ProtocolAdapter: std::fmt::Debug + Send + Sync {
    fn protocol_name(&self) -> sql_lens_core::ProtocolName;
    fn create_connection_state(&self, context: &ProtocolConnectionContext) -> Box<dyn ProtocolConnectionState>;
    fn observe_client_bytes(
        &self,
        state: &mut dyn ProtocolConnectionState,
        bytes: &[u8],
        events: &mut dyn CaptureEventEmitter,
    ) -> Result<ProtocolObservation, ProtocolAdapterError>;
    fn observe_backend_bytes(
        &self,
        state: &mut dyn ProtocolConnectionState,
        bytes: &[u8],
        events: &mut dyn CaptureEventEmitter,
    ) -> Result<ProtocolObservation, ProtocolAdapterError>;
}

pub trait ProtocolConnectionState: std::any::Any + std::fmt::Debug + Send {
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

pub trait CaptureEventEmitter {
    fn emit(&mut self, event: sql_lens_core::SqlEvent);
}

pub struct ProtocolAdapterRegistry;

impl ProtocolAdapterRegistry {
    pub fn new() -> Self;
    pub fn register<A>(&mut self, adapter: A) -> Result<(), ProtocolAdapterRegistryError>
    where
        A: ProtocolAdapter + 'static;
    pub fn register_shared(&mut self, adapter: std::sync::Arc<dyn ProtocolAdapter>) -> Result<(), ProtocolAdapterRegistryError>;
    pub fn resolve(&self, protocol: &sql_lens_core::ProtocolName) -> Result<std::sync::Arc<dyn ProtocolAdapter>, ProtocolAdapterRegistryError>;
    pub fn contains(&self, protocol: &sql_lens_core::ProtocolName) -> bool;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
}
```

Allowed dependency:

```toml
sql-lens-core = { path = "../sql-lens-core" }
```

Do not add `tokio`, `async-trait`, `sql-lens-capture`, proxy, storage, API, app, `thiserror`, `anyhow`, or protocol-specific crates for this contract layer.

### 3. Contracts

- `ProtocolAdapter` must be object-safe. Do not add generic methods or associated types.
- Per-connection parser state is represented as `Box<dyn ProtocolConnectionState>` so heterogeneous adapters can be stored in one registry later.
- Concrete adapters downcast state through `as_any_mut().downcast_mut::<AdapterState>()`.
- `observe_client_bytes` observes client-to-backend bytes.
- `observe_backend_bytes` observes backend-to-client bytes.
- Adapters emit normalized `SqlEvent` values through `CaptureEventEmitter`.
- Capture channel overload policy is outside this crate; adapter parsing should not depend on runtime channel behavior.
- `ProtocolObservation.bytes_observed` records input bytes seen by the adapter.
- `ProtocolObservation.events_emitted` records events emitted through the emitter.
- `ProtocolAdapterRegistry` stores adapters keyed by `ProtocolName`.
- Registry storage uses `Arc<dyn ProtocolAdapter>` so resolved adapters can be shared by runtime tasks.
- Unknown adapter names return `ProtocolAdapterRegistryError::UnknownAdapter`.
- Duplicate adapter names return `ProtocolAdapterRegistryError::DuplicateAdapter`.
- Config validation mapping is a later composition task; do not make `sql-lens-config` depend on `sql-lens-protocol`.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| Adapter needs connection state | `create_connection_state` returns `Box<dyn ProtocolConnectionState>` |
| Adapter receives expected state type | Downcast succeeds and observation updates state |
| Adapter receives wrong state type | Return `ProtocolAdapterError::InvalidConnectionState` |
| Client bytes are observed | Return observed byte count and emitted event count |
| Backend bytes are observed | Return observed byte count and emitted event count |
| Adapter emits SQL event | Call `CaptureEventEmitter::emit(SqlEvent)` |
| Registry needs trait objects | `Box<dyn ProtocolAdapter>` compiles and can observe bytes |
| Adapter is registered | Registry resolves the same protocol name to an `Arc<dyn ProtocolAdapter>` |
| Adapter protocol name is duplicated | Return `DuplicateAdapter` |
| Adapter protocol name is unknown | Return `UnknownAdapter` |

### 5. Good/Base/Bad Cases

Good:

- A MySQL adapter owns a MySQL-specific state struct but exposes it as `Box<dyn ProtocolConnectionState>`.
- A test adapter proves `Box<dyn ProtocolAdapter>` works before the registry task starts.

Base:

- Unit tests use dummy bytes and synthetic `SqlEvent` values.
- Invalid state errors are structured without adding third-party error crates.
- Registry errors are structured in protocol crate and mapped to user-facing config errors later.

Bad:

- Defining `trait ProtocolAdapter<State>` or an associated `type State`, which prevents a heterogeneous registry without another erasure layer.
- Importing `sql-lens-capture` and making parsers depend on channel overload behavior.
- Importing `sql-lens-protocol` from `sql-lens-config` just to validate startup protocol names.
- Adding async trait methods before parser work proves it is needed.
- Putting MySQL-specific packet fields in protocol-neutral contracts.

### 6. Tests Required

For protocol adapter contract changes:

- Trait object usage test with `Box<dyn ProtocolAdapter>`.
- Client byte observation test.
- Backend byte observation test.
- Event emission test.
- Protocol-specific state downcast test.
- Registry register/resolve test.
- Registry unknown adapter test.
- Registry duplicate adapter test.
- Structured error display/source test.
- Run `cargo fmt --check`.
- Run `cargo check --workspace`.
- Run `cargo test --workspace`.
- Run `cargo clippy --workspace --all-targets -- -D warnings`.

### 7. Wrong vs Correct

#### Wrong

```rust
pub trait ProtocolAdapter {
    type State;
    fn observe(&self, state: &mut Self::State, bytes: &[u8]);
}
```

#### Correct

```rust
pub trait ProtocolAdapter {
    fn create_connection_state(&self, context: &ProtocolConnectionContext) -> Box<dyn ProtocolConnectionState>;
    fn observe_client_bytes(&self, state: &mut dyn ProtocolConnectionState, bytes: &[u8], events: &mut dyn CaptureEventEmitter) -> Result<ProtocolObservation, ProtocolAdapterError>;
}
```

> Gotcha: when downcasting a boxed state in callers or tests, use `state.as_ref().as_any()` or `state.as_mut().as_any_mut()`. Calling `state.as_any()` directly on `Box<dyn ProtocolConnectionState>` can target the box's blanket implementation instead of the inner state.

## Scenario: MySQL Protocol Adapter Foundation

### 1. Scope / Trigger

- Trigger: `sql-lens-protocol-mysql` provides the first concrete protocol adapter crate.
- The first MySQL adapter foundation must prove registry integration without implementing packet parsing.
- MySQL-specific parser state belongs inside `sql-lens-protocol-mysql`, not in protocol-neutral crates.

### 2. Signatures

Public MySQL adapter types live in `crates/sql-lens-protocol-mysql/src/lib.rs`:

```rust
pub const MYSQL_PROTOCOL_NAME: &str = "mysql";

pub struct MysqlProtocolAdapter;

impl MysqlProtocolAdapter {
    pub fn new() -> Self;
}

pub struct MysqlConnectionState;

impl MysqlConnectionState {
    pub fn client_bytes_observed(&self) -> usize;
    pub fn backend_bytes_observed(&self) -> usize;
}
```

Allowed dependencies:

```toml
sql-lens-core = { path = "../sql-lens-core" }
sql-lens-protocol = { path = "../sql-lens-protocol" }
```

### 3. Contracts

- `MysqlProtocolAdapter::protocol_name()` returns `ProtocolName("mysql")`.
- `create_connection_state` returns boxed `MysqlConnectionState`.
- `observe_client_bytes` downcasts to `MysqlConnectionState`, increments observed client bytes, returns `ProtocolObservation::new(bytes.len(), 0)`, and emits no events.
- `observe_backend_bytes` downcasts to `MysqlConnectionState`, increments observed backend bytes, returns `ProtocolObservation::new(bytes.len(), 0)`, and emits no events.
- Wrong state type returns `ProtocolAdapterError::InvalidConnectionState { expected: "MysqlConnectionState" }`.
- The adapter can be registered and resolved through `ProtocolAdapterRegistry`.
- Packet framing, handshake observation, authentication, commands, prepared statements, parameter decoding, and event emission belong to later MySQL protocol tasks.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| Adapter protocol name requested | Return `mysql` |
| Adapter registered in registry | Resolve succeeds for `ProtocolName("mysql")` |
| Connection state created | State downcasts to `MysqlConnectionState` |
| Client bytes observed | Return byte count and zero emitted events |
| Backend bytes observed | Return byte count and zero emitted events |
| Wrong connection state passed | Return `InvalidConnectionState` |

### 5. Good/Base/Bad Cases

Good:

- The adapter is a no-op parser foundation that proves crate wiring and registry compatibility.
- Tests assert zero emitted events until parsing tasks exist.

Base:

- Future packet parsing can extend `MysqlConnectionState` without changing the shared adapter trait.

Bad:

- Emitting placeholder SQL events from raw bytes before MySQL parsing exists.
- Adding packet header parsing in the adapter foundation task.
- Adding dependencies on proxy, capture, storage, API, app, or async runtime crates.

### 6. Tests Required

For MySQL adapter foundation changes:

- Protocol name test.
- Registry register/resolve test.
- State downcast test.
- Client/backend byte observation test.
- Wrong-state error test.
- Run `cargo fmt --check`.
- Run `cargo check --workspace`.
- Run `cargo test --workspace`.
- Run `cargo clippy --workspace --all-targets -- -D warnings`.

## Scenario: MySQL Packet Header Parser Contracts

### 1. Scope / Trigger

- Trigger: `sql-lens-protocol-mysql` parses MySQL-compatible packet envelopes.
- This parser owns the 4-byte packet header only.
- It must not parse handshake payloads, commands, result packets, prepared statements, or emit SQL events.

### 2. Signatures

Public packet parser types live in `crates/sql-lens-protocol-mysql/src/packet.rs` and are re-exported from the crate root:

```rust
pub const MYSQL_PACKET_HEADER_LEN: usize = 4;

pub struct MysqlPacketHeader {
    pub payload_length: u32,
    pub sequence_id: u8,
}

pub struct MysqlPacket<'a> {
    pub header: MysqlPacketHeader,
    pub payload: &'a [u8],
}

pub fn parse_mysql_packet(input: &[u8]) -> Result<MysqlPacket<'_>, MysqlPacketParseError>;

pub enum MysqlPacketParseError {
    IncompleteHeader { available: usize },
    IncompletePayload { declared: u32, available: usize },
}
```

### 3. Contracts

- MySQL packet headers are exactly 4 bytes.
- Header bytes `0..3` encode payload length as a 3-byte little-endian unsigned integer.
- Header byte `3` encodes sequence ID.
- Payload length excludes the 4-byte header.
- Successful parsing returns a borrowed payload slice and performs no allocation.
- If input has fewer than 4 bytes, return `IncompleteHeader`.
- If declared payload length exceeds available payload bytes, return `IncompletePayload`.
- Trailing bytes after the first complete packet are ignored by this single-packet parser.
- Stream buffering, reassembly, and multi-packet parsing belong to later tasks.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| Normal packet with payload | Return payload length, sequence ID, and payload slice |
| Empty-payload packet | Return payload length `0` and empty payload slice |
| Header shorter than 4 bytes | Return `IncompleteHeader { available }` |
| Declared payload length exceeds available bytes | Return `IncompletePayload { declared, available }` |
| Extra bytes follow a complete packet | Returned payload excludes trailing bytes |

### 5. Good/Base/Bad Cases

Good:

- Packet parser tests use inline byte arrays for normal and malformed packets.
- Golden fixture tests use ASCII hex files under `crates/sql-lens-protocol-mysql/fixtures/packets/`.
- Errors implement `Display` and `Error` without third-party error crates.

Base:

- Later stream framing can repeatedly call `parse_mysql_packet` on retained buffers.
- Fixture files may use spaces, newlines, and `#` comments; they represent raw packet bytes passed directly to `parse_mysql_packet`.

Bad:

- Parsing payload contents inside the header parser.
- Allocating a payload buffer on successful parse.
- Emitting SQL events from packet parser tests.
- Creating binary fixture decoders or packet reassembly helpers in the packet header parser task.

### 6. Tests Required

For MySQL packet parser changes:

- Normal packet test.
- Empty payload test.
- 3-byte little-endian payload length test.
- Short header test.
- Incomplete payload test.
- Trailing bytes test.
- Error display test.
- Fixture tests for normal, empty-payload, short-header, and incomplete-payload packets.
- Fixture format documented in `crates/sql-lens-protocol-mysql/fixtures/packets/README.md`.
- Run `cargo fmt --check`.
- Run `cargo test -p sql-lens-protocol-mysql`.
- Run `cargo test --workspace`.
- Run `cargo clippy --workspace --all-targets -- -D warnings`.

## Scenario: MySQL Initial Handshake Observation Contracts

### 1. Scope / Trigger

- Trigger: `sql-lens-protocol-mysql` observes the backend-to-client MySQL-compatible initial handshake packet.
- This layer decodes protocol setup metadata needed by later authentication and command parsing tasks.
- It must not store authentication challenge bytes, parse client authentication responses, emit SQL events, log packet payloads, or add stream buffering.

### 2. Signatures

Public handshake parser types live in `crates/sql-lens-protocol-mysql/src/handshake.rs` and are re-exported from the crate root:

```rust
pub struct MysqlInitialHandshake {
    pub protocol_version: u8,
    pub server_version: String,
    pub connection_id: u32,
    pub capability_flags: Option<u32>,
    pub character_set: Option<u8>,
    pub status_flags: Option<u16>,
    pub auth_plugin_name: Option<String>,
}

pub fn parse_initial_handshake(
    payload: &[u8],
) -> Result<MysqlInitialHandshake, MysqlHandshakeParseError>;

pub enum MysqlHandshakeParseError {
    EmptyPayload,
    UnsupportedProtocolVersion { version: u8 },
    MissingServerVersionTerminator,
    IncompletePayload { field: &'static str, needed: usize, available: usize },
    InvalidUtf8 { field: &'static str },
}

pub enum MysqlConnectionPhase {
    AwaitingInitialHandshake,
    InitialHandshakeSeen,
    ClientHandshakeSeen,
}
```

`MysqlConnectionState` exposes read-only state accessors:

```rust
impl MysqlConnectionState {
    pub fn phase(&self) -> MysqlConnectionPhase;
    pub fn initial_handshake(&self) -> Option<&MysqlInitialHandshake>;
}
```

### 3. Contracts

- `MysqlConnectionState::default()` starts in `AwaitingInitialHandshake`.
- The initial handshake is observed only from backend-to-client bytes.
- `observe_backend_bytes` continues to return `ProtocolObservation::new(bytes.len(), 0)` for this layer.
- `observe_backend_bytes` attempts handshake observation only while the phase is `AwaitingInitialHandshake`.
- A complete packet with sequence ID `0` and a valid Protocol 10 payload stores sanitized handshake metadata and moves the phase to `InitialHandshakeSeen`.
- Client bytes do not move the phase to `InitialHandshakeSeen`.
- Incomplete or malformed backend bytes remain non-fatal and keep the phase as `AwaitingInitialHandshake`; stream buffering belongs to a later task.
- `MysqlInitialHandshake` must not contain auth plugin challenge/scramble bytes.
- The parser may expose safe setup metadata: protocol version, server version, connection ID, capability flags, character set, status flags, and auth plugin name.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| Empty handshake payload | Return `EmptyPayload` |
| Protocol version is not 10 | Return `UnsupportedProtocolVersion` |
| Server version lacks NUL terminator | Return `MissingServerVersionTerminator` |
| Required field is incomplete | Return `IncompletePayload { field, needed, available }` |
| Server version or plugin name is invalid UTF-8 | Return `InvalidUtf8 { field }` |
| Complete backend sequence-0 handshake | Store sanitized metadata; set phase to `InitialHandshakeSeen`; emit zero events |
| Client sends handshake-shaped bytes | Count bytes only; keep phase awaiting |
| Backend sends incomplete/malformed bytes through adapter | Count bytes only; keep phase awaiting; emit zero events |

### 5. Good/Base/Bad Cases

Good:

- Parser tests use representative Protocol 10 bytes and assert safe metadata fields.
- Adapter tests publish a complete packet through `observe_backend_bytes` and assert state transition plus zero events.
- Debug output for `MysqlInitialHandshake` does not include auth challenge strings because they are never stored.

Base:

- Later authentication tasks can read stored capability flags and auth plugin name without re-parsing the initial handshake.
- Later stream buffering can call the same parser after assembling complete packets.

Bad:

- Storing `auth_plugin_data_part_1`, `auth_plugin_data_part_2`, or raw handshake payload bytes.
- Logging raw handshake bytes or server challenge data.
- Failing packet forwarding because an observed handshake is malformed.
- Parsing client handshake response in the initial-handshake task.

### 6. Tests Required

For MySQL initial handshake changes:

- Parser test for representative Protocol 10 handshake metadata.
- Parser tests for empty payload, unsupported protocol version, missing server-version terminator, incomplete required fields, and invalid UTF-8.
- Test that auth challenge bytes are not exposed by the parsed handshake type.
- State creation test for `AwaitingInitialHandshake`.
- Adapter backend handshake transition test.
- Adapter client-bytes no-transition test.
- Adapter malformed-backend-bytes non-fatal test.
- Run `cargo fmt --check`.
- Run `cargo test -p sql-lens-protocol-mysql`.
- Run `cargo test --workspace`.
- Run `cargo clippy --workspace --all-targets -- -D warnings`.

### 7. Wrong vs Correct

#### Wrong

```rust
pub struct MysqlInitialHandshake {
    pub auth_plugin_data_part_1: Vec<u8>,
    pub raw_payload: Vec<u8>,
}
```

#### Correct

```rust
pub struct MysqlInitialHandshake {
    pub protocol_version: u8,
    pub server_version: String,
    pub connection_id: u32,
    pub auth_plugin_name: Option<String>,
}
```

## Scenario: MySQL Client Handshake Response Observation Contracts

### 1. Scope / Trigger

- Trigger: `sql-lens-protocol-mysql` observes the client-to-backend MySQL-compatible handshake response after the initial server handshake.
- This layer extracts safe authentication metadata needed by later auth-result and command-parsing tasks.
- It must not store authentication response bytes, decode passwords, handle TLS/SSLRequest, emit SQL events, update shared connection models, log payloads, or add stream buffering.

### 2. Signatures

Public client handshake parser types live in `crates/sql-lens-protocol-mysql/src/handshake.rs` and are re-exported from the crate root:

```rust
pub struct MysqlClientHandshakeResponse {
    pub capability_flags: u32,
    pub max_packet_size: u32,
    pub character_set: u8,
    pub username: Option<String>,
    pub database: Option<String>,
    pub auth_plugin_name: Option<String>,
}

pub fn parse_client_handshake_response(
    payload: &[u8],
) -> Result<MysqlClientHandshakeResponse, MysqlClientHandshakeParseError>;

pub enum MysqlClientHandshakeParseError {
    IncompletePayload { field: &'static str, needed: usize, available: usize },
    UnsupportedProtocol { message: &'static str },
    MissingNullTerminator { field: &'static str },
    InvalidUtf8 { field: &'static str },
    InvalidLengthEncodedInteger { field: &'static str },
}
```

`MysqlConnectionState` exposes safe client response metadata:

```rust
impl MysqlConnectionState {
    pub fn client_handshake(&self) -> Option<&MysqlClientHandshakeResponse>;
}
```

### 3. Contracts

- Client handshake response observation happens only when phase is `InitialHandshakeSeen`.
- `observe_client_bytes` always increments observed client bytes and returns `ProtocolObservation::new(bytes.len(), 0)` for this layer.
- A complete client packet with sequence ID `1` and a valid Protocol 41 response stores sanitized metadata and moves phase to `ClientHandshakeSeen`.
- Client response-shaped bytes before the initial server handshake do not update client handshake state.
- Incomplete or malformed client bytes remain non-fatal and keep the previous phase.
- The parser requires `CLIENT_PROTOCOL_41`.
- SSLRequest packets are reported as unsupported full client handshake responses; TLS handling belongs to a later task.
- Safe metadata may include capability flags, max packet size, character set, username, database, and auth plugin name.
- Auth response bytes are skipped according to client capability flags and must not appear on `MysqlClientHandshakeResponse`.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| Fixed Protocol 41 header is incomplete | Return `IncompletePayload` |
| `CLIENT_PROTOCOL_41` is missing | Return `UnsupportedProtocol` |
| SSLRequest shape is observed | Return `UnsupportedProtocol`; adapter keeps prior phase |
| Username/database/plugin lacks NUL terminator | Return `MissingNullTerminator { field }` |
| Username/database/plugin is invalid UTF-8 | Return `InvalidUtf8 { field }` |
| Length-encoded auth response length marker is invalid | Return `InvalidLengthEncodedInteger { field: "auth_response" }` |
| Complete client sequence-1 response after initial handshake | Store sanitized metadata; set phase to `ClientHandshakeSeen`; emit zero events |
| Client response before initial handshake | Count bytes only; keep phase awaiting; emit zero events |
| Malformed client response after initial handshake | Count bytes only; keep phase `InitialHandshakeSeen`; emit zero events |

### 5. Good/Base/Bad Cases

Good:

- Parser tests assert username, database, plugin, and capability metadata while checking debug output does not include auth response strings.
- Adapter tests first observe a backend initial handshake, then observe a client response and assert the phase transition.
- Secure-connection and length-encoded auth response forms are skipped without storing bytes.

Base:

- Later auth-result detection can start from `ClientHandshakeSeen`.
- Later connection/API layers can decide whether and how to project sanitized username/database into shared connection records.

Bad:

- Adding an `auth_response`, `password`, `raw_payload`, or `packet_bytes` field to `MysqlClientHandshakeResponse`.
- Logging username/password pairs or raw handshake response bytes.
- Treating SSLRequest as authenticated or as a full client handshake response.
- Emitting SQL events from authentication observation.

### 6. Tests Required

For MySQL client handshake response changes:

- Parser test for Protocol 41 response with username, database, and plugin name.
- Parser test without database/plugin flags.
- Parser tests for secure-connection and length-encoded auth response skipping.
- Parser tests for incomplete header, missing NUL terminator, invalid UTF-8, and invalid length-encoded auth length.
- Test that auth response bytes are not exposed by the parsed response type.
- Adapter transition test from `InitialHandshakeSeen` to `ClientHandshakeSeen`.
- Adapter test proving client response before initial handshake does not transition.
- Adapter malformed-client-response non-fatal test.
- Run `cargo fmt --check`.
- Run `cargo test -p sql-lens-protocol-mysql`.
- Run `cargo test --workspace`.
- Run `cargo clippy --workspace --all-targets -- -D warnings`.

### 7. Wrong vs Correct

#### Wrong

```rust
pub struct MysqlClientHandshakeResponse {
    pub username: Option<String>,
    pub auth_response: Vec<u8>,
}
```

#### Correct

```rust
pub struct MysqlClientHandshakeResponse {
    pub username: Option<String>,
    pub database: Option<String>,
    pub auth_plugin_name: Option<String>,
}
```

## Scenario: MySQL Authentication Result Observation Contracts

### 1. Scope / Trigger

- Trigger: `sql-lens-protocol-mysql` observes backend-to-client authentication result packets after the client handshake response.
- This layer records the MySQL-specific authentication outcome needed before later command parsing starts.
- It must not parse SQL commands, implement auth switch flows, store raw authentication payloads, log backend auth packets, emit SQL events, or update protocol-neutral connection models.

### 2. Signatures

Public authentication result parser types live in `crates/sql-lens-protocol-mysql/src/authentication.rs` and are re-exported from the crate root:

```rust
pub enum MysqlAuthenticationStatus {
    Succeeded,
    Failed,
}

pub struct MysqlAuthenticationResult {
    pub status: MysqlAuthenticationStatus,
    pub error_code: Option<u16>,
    pub sql_state: Option<String>,
    pub message: Option<String>,
}

pub fn parse_authentication_result(
    payload: &[u8],
) -> Result<Option<MysqlAuthenticationResult>, MysqlAuthenticationResultParseError>;

pub enum MysqlAuthenticationResultParseError {
    IncompletePayload { field: &'static str, needed: usize, available: usize },
    InvalidUtf8 { field: &'static str },
}
```

`MysqlConnectionState` exposes safe authentication result metadata:

```rust
impl MysqlConnectionState {
    pub fn authentication_result(&self) -> Option<&MysqlAuthenticationResult>;
}
```

`MysqlConnectionPhase` includes post-authentication phases:

```rust
pub enum MysqlConnectionPhase {
    AwaitingInitialHandshake,
    InitialHandshakeSeen,
    ClientHandshakeSeen,
    Authenticated,
    AuthenticationFailed,
}
```

### 3. Contracts

- Authentication result observation happens only when phase is `ClientHandshakeSeen`.
- `observe_backend_bytes` always increments observed backend bytes and returns `ProtocolObservation::new(bytes.len(), 0)`.
- A backend OK packet payload starting with `0x00` stores a success result and moves phase to `Authenticated`.
- A backend ERR packet payload starting with `0xff` stores a failure result and moves phase to `AuthenticationFailed`.
- Failure metadata may include MySQL vendor error code, SQL state, and a UTF-8 database error message.
- Raw backend authentication payload bytes are never stored.
- Unsupported authentication continuation packets return `Ok(None)` from the parser and keep phase `ClientHandshakeSeen`.
- Incomplete or malformed authentication result packets remain non-fatal and keep phase `ClientHandshakeSeen`.
- Authentication result observation emits zero SQL events.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| Empty auth result payload | Return `IncompletePayload { field: "header" }` |
| Payload starts with `0x00` | Return success result |
| Payload starts with `0xff` and has ERR metadata | Return failure result with safe metadata |
| ERR payload has invalid UTF-8 SQL state or message | Return `InvalidUtf8` |
| Payload starts with any other byte | Return `Ok(None)` |
| OK/ERR before `ClientHandshakeSeen` | Count bytes only; keep prior phase; emit zero events |
| OK after `ClientHandshakeSeen` | Store success; set phase to `Authenticated`; emit zero events |
| ERR after `ClientHandshakeSeen` | Store failure; set phase to `AuthenticationFailed`; emit zero events |
| Unsupported auth continuation after `ClientHandshakeSeen` | Keep phase `ClientHandshakeSeen`; emit zero events |

### 5. Good/Base/Bad Cases

Good:

- Parser tests cover OK, ERR, unsupported packet headers, malformed payloads, and invalid UTF-8.
- Adapter tests first observe server handshake and client response, then observe backend auth results.
- Authentication failure debug data contains only structured safe fields, never raw packet bytes.

Base:

- Later command parsing can start only once the phase is `Authenticated`.
- Later auth switch work can refine continuation handling without changing the safe result type.

Bad:

- Treating auth switch request packets as authenticated.
- Storing raw OK/ERR packet bytes, auth plugin challenge data, or client auth response data.
- Emitting SQL events from authentication observation.
- Adding MySQL auth fields to protocol-neutral `ConnectionInfo`.

### 6. Tests Required

For MySQL authentication result changes:

- Parser test for OK success result.
- Parser test for ERR failure metadata.
- Parser test for unsupported continuation packet returning `None`.
- Parser tests for empty payload and invalid UTF-8.
- Adapter test that backend OK after client handshake moves phase to `Authenticated`.
- Adapter test that backend ERR after client handshake moves phase to `AuthenticationFailed`.
- Adapter test proving auth-shaped backend bytes before client handshake do not transition.
- Adapter test proving unsupported auth continuation stays non-fatal and non-transitioning.
- Run `cargo fmt --check`.
- Run `cargo test -p sql-lens-protocol-mysql`.
- Run `cargo test --workspace`.
- Run `cargo clippy --workspace --all-targets -- -D warnings`.

### 7. Wrong vs Correct

#### Wrong

```rust
pub struct MysqlAuthenticationResult {
    pub raw_payload: Vec<u8>,
}
```

#### Correct

```rust
pub struct MysqlAuthenticationResult {
    pub status: MysqlAuthenticationStatus,
    pub error_code: Option<u16>,
    pub sql_state: Option<String>,
    pub message: Option<String>,
}
```

## Scenario: MySQL COM_QUERY Parser Contracts

### 1. Scope / Trigger

- Trigger: `sql-lens-protocol-mysql` parses client-to-backend MySQL command packets after authentication.
- This layer records safe MySQL-specific command metadata needed by later query timing and SQL event capture tasks.
- It must not measure duration, inspect backend responses, emit SQL events, normalize SQL, fingerprint SQL, redact SQL, or update protocol-neutral core models.

### 2. Signatures

Public command parser types live in `crates/sql-lens-protocol-mysql/src/command.rs` and are re-exported from the crate root:

```rust
pub const MYSQL_COM_QUERY: u8 = 0x03;
pub const MYSQL_COM_STMT_PREPARE: u8 = 0x16;

pub enum MysqlCommandKind {
    Query,
    StatementPrepare,
}

pub struct MysqlComQuery {
    pub sql: String,
}

pub struct MysqlComStmtPrepare {
    pub template_sql: String,
}

pub enum MysqlParsedClientCommand {
    Query(MysqlComQuery),
    StatementPrepare(MysqlComStmtPrepare),
}

pub struct MysqlClientCommand {
    pub kind: MysqlCommandKind,
    pub sequence_id: u8,
    pub sql: String,
}

pub fn parse_client_command(
    payload: &[u8],
) -> Result<Option<MysqlParsedClientCommand>, MysqlCommandParseError>;

pub enum MysqlCommandParseError {
    IncompletePayload { field: &'static str, needed: usize, available: usize },
    InvalidUtf8 { field: &'static str },
}
```

`MysqlConnectionState` exposes the latest parsed command metadata:

```rust
impl MysqlConnectionState {
    pub fn last_client_command(&self) -> Option<&MysqlClientCommand>;
}
```

### 3. Contracts

- Client command observation happens only when phase is `Authenticated`.
- `observe_client_bytes` always increments observed client bytes and returns `ProtocolObservation::new(bytes.len(), 0)`.
- A payload starting with `MYSQL_COM_QUERY` parses the remaining payload bytes as UTF-8 SQL text.
- Empty SQL text is valid and stored as an empty string; server-side rejection belongs to later response handling.
- Invalid UTF-8 SQL text returns `MysqlCommandParseError::InvalidUtf8 { field: "sql" }`.
- Empty command payload returns `MysqlCommandParseError::IncompletePayload { field: "command" }`.
- Unsupported command bytes return `Ok(None)`.
- Adapter observation treats unsupported, incomplete, malformed, and invalid-UTF-8 command packets as non-fatal and non-transitioning.
- Parsed command state is MySQL-specific and must not be projected into `SqlEvent` or `ConnectionInfo` in this task.
- Command observation emits zero SQL events.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| Empty command payload | Return `IncompletePayload { field: "command" }` |
| Payload starts with `0x03` and valid UTF-8 SQL | Return `Some(MysqlParsedClientCommand::Query(MysqlComQuery))` |
| Payload is exactly `[0x03]` | Return `Some(MysqlParsedClientCommand::Query(MysqlComQuery { sql: "" }))` |
| Payload starts with `0x03` and invalid UTF-8 SQL | Return `InvalidUtf8 { field: "sql" }` |
| Payload starts with another command byte | Return `Ok(None)` |
| COM_QUERY before `Authenticated` | Count bytes only; do not update command state; emit zero events |
| COM_QUERY after `Authenticated` | Store `MysqlClientCommand { kind: Query, sequence_id, sql }`; keep phase `Authenticated`; emit zero events |
| Unsupported command after `Authenticated` | Keep phase `Authenticated`; do not update command state; emit zero events |
| Invalid command packet after `Authenticated` | Keep phase `Authenticated`; do not update command state; emit zero events |

### 5. Good/Base/Bad Cases

Good:

- Parser tests cover valid SQL text, empty SQL text, unsupported commands, empty payloads, and invalid UTF-8.
- Adapter tests first authenticate the connection, then observe command packets.
- The packet envelope sequence ID is retained only in MySQL-specific command metadata.

Base:

- Query timing work stores a separate pending-query slot while preserving the standalone parser contract.
- Later character-set support can refine SQL decoding while preserving non-fatal adapter behavior.

Bad:

- Emitting `SqlEvent` from command parsing before timing and backend response finalization exist.
- Adding SQL text fields to protocol-neutral connection models.
- Logging raw SQL text from parser or adapter code.
- Blocking forwarding on storage, UI, plugin, or capture pipeline work.
- Parsing backend responses or emitting events from the command parser.

### 6. Tests Required

For MySQL `COM_QUERY` parser changes:

- Parser test for valid `COM_QUERY` SQL text.
- Parser test for empty `COM_QUERY` SQL text.
- Parser test for unsupported command byte.
- Parser tests for empty payload and invalid UTF-8.
- Adapter test proving `COM_QUERY` before authentication does not update command state.
- Adapter test proving `COM_QUERY` after authentication stores kind, sequence ID, and SQL text.
- Adapter test proving unsupported commands after authentication stay non-fatal.
- Adapter test proving invalid or malformed command packets after authentication stay non-fatal.
- Run `cargo fmt --check`.
- Run `cargo test -p sql-lens-protocol-mysql`.
- Run `cargo test --workspace`.
- Run `cargo clippy --workspace --all-targets -- -D warnings`.

### 7. Wrong vs Correct

#### Wrong

```rust
pub struct SqlEvent {
    pub mysql_command: u8,
    pub sql: String,
}
```

#### Correct

```rust
pub struct MysqlClientCommand {
    pub kind: MysqlCommandKind,
    pub sequence_id: u8,
    pub sql: String,
}
```

## Scenario: MySQL COM_STMT_PREPARE Parser Contracts

### 1. Scope / Trigger

- Trigger: `sql-lens-protocol-mysql` parses client-to-backend `COM_STMT_PREPARE` command packets after authentication.
- This layer records the prepared statement SQL template before later backend response parsing can map a server statement ID.
- It must not parse backend `COM_STMT_PREPARE_OK`, allocate or map statement IDs, parse parameter definition packets, expand parameters, emit SQL events, or update protocol-neutral core models.

### 2. Signatures

Public prepared-statement command parser types live in `crates/sql-lens-protocol-mysql/src/command.rs` and are re-exported from the crate root:

```rust
pub const MYSQL_COM_STMT_PREPARE: u8 = 0x16;

pub enum MysqlCommandKind {
    Query,
    StatementPrepare,
}

pub struct MysqlComStmtPrepare {
    pub template_sql: String,
}

pub enum MysqlParsedClientCommand {
    Query(MysqlComQuery),
    StatementPrepare(MysqlComStmtPrepare),
}
```

`MysqlConnectionState` exposes MySQL-local pending prepare state:

```rust
pub struct MysqlPendingStatementPrepare {
    pub command: MysqlClientCommand,
}

impl MysqlConnectionState {
    pub fn pending_statement_prepare(&self) -> Option<&MysqlPendingStatementPrepare>;
}
```

### 3. Contracts

- Client command observation happens only when phase is `Authenticated`.
- A payload starting with `MYSQL_COM_STMT_PREPARE` parses the remaining payload bytes as UTF-8 SQL template text.
- Empty template text is valid and stored as an empty string; server-side rejection belongs to backend response handling.
- Invalid UTF-8 template text returns `MysqlCommandParseError::InvalidUtf8 { field: "template_sql" }`.
- Adapter observation treats unsupported, incomplete, malformed, and invalid-UTF-8 prepare packets as non-fatal and non-transitioning.
- Valid `COM_STMT_PREPARE` after authentication stores `last_client_command` with `kind = MysqlCommandKind::StatementPrepare`, the packet envelope sequence ID, and the template SQL in the existing `sql` field.
- Valid `COM_STMT_PREPARE` after authentication stores `MysqlPendingStatementPrepare` and replaces any existing pending prepare record.
- Prepare command parsing emits zero SQL events and must not call timing-only logic intended for `COM_QUERY`.
- Parsed prepare state is MySQL-specific and must not be projected into `SqlEvent`, `ConnectionInfo`, storage, API, WebSocket, or plugin contracts in this layer.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| Payload starts with `0x16` and valid UTF-8 template | Return `Some(MysqlParsedClientCommand::StatementPrepare(MysqlComStmtPrepare))` |
| Payload is exactly `[0x16]` | Return `Some(MysqlParsedClientCommand::StatementPrepare(MysqlComStmtPrepare { template_sql: "" }))` |
| Payload starts with `0x16` and invalid UTF-8 template | Return `InvalidUtf8 { field: "template_sql" }` |
| `COM_STMT_PREPARE` before `Authenticated` | Count bytes only; do not update command or pending prepare state; emit zero events |
| `COM_STMT_PREPARE` after `Authenticated` | Store command and pending prepare state; keep phase `Authenticated`; emit zero events |
| Invalid prepare packet after `Authenticated` | Keep phase `Authenticated`; do not update command or pending prepare state; emit zero events |
| Backend prepare response is observed | Follow the MySQL `COM_STMT_PREPARE` response parser contract; the client command parser must not parse it |

### 5. Good/Base/Bad Cases

Good:

- Parser tests cover valid template SQL, empty template SQL, and invalid UTF-8 template bytes.
- Adapter tests cover prepare before authentication, prepare after authentication, and invalid prepare bytes after authentication.
- The packet envelope sequence ID is retained only in MySQL-specific command metadata.

Base:

- Later backend response parsing can consume `MysqlPendingStatementPrepare` to map a server `statement_id`.
- Later parameter expansion can use the stored template SQL after `COM_STMT_EXECUTE` decoding exists.

Bad:

- Parsing backend `COM_STMT_PREPARE_OK` in the client command parser.
- Adding statement IDs before the backend response parser exists.
- Emitting a protocol-neutral SQL event from prepare command observation alone.
- Logging raw template SQL text from parser or adapter code.
- Adding MySQL prepared-statement fields directly to shared core models.

### 6. Tests Required

For MySQL `COM_STMT_PREPARE` parser changes:

- Parser test for valid `COM_STMT_PREPARE` template SQL.
- Parser test for empty `COM_STMT_PREPARE` template SQL.
- Parser test for invalid UTF-8 template SQL.
- Adapter test proving `COM_STMT_PREPARE` before authentication does not update state.
- Adapter test proving `COM_STMT_PREPARE` after authentication stores kind, sequence ID, and template SQL.
- Adapter test proving invalid prepare command bytes after authentication stay non-fatal.
- Regression test coverage that existing `COM_QUERY` behavior remains unchanged.
- Run `cargo fmt --check`.
- Run `cargo test -p sql-lens-protocol-mysql`.
- Run `cargo test --workspace`.
- Run `cargo clippy --workspace --all-targets -- -D warnings`.

### 7. Wrong vs Correct

#### Wrong

```rust
pub struct MysqlPendingStatementPrepare {
    pub statement_id: u32,
    pub template_sql: String,
}
```

#### Correct

```rust
pub struct MysqlPendingStatementPrepare {
    pub command: MysqlClientCommand,
}
```

## Scenario: MySQL COM_STMT_PREPARE Response Parser Contracts

### 1. Scope / Trigger

- Trigger: `sql-lens-protocol-mysql` observes backend-to-client responses after a valid client `COM_STMT_PREPARE`.
- This layer parses the first prepare OK response packet or ERR packet so later statement-map work can connect server `statement_id` values to SQL templates.
- It must not build the per-connection statement map, parse parameter definition packets, parse column definition packets, expand parameters, emit SQL events, or update protocol-neutral core models.

### 2. Signatures

Public prepare response parser types live in `crates/sql-lens-protocol-mysql/src/prepare.rs` and are re-exported from the crate root:

```rust
pub struct MysqlComStmtPrepareOk {
    pub statement_id: u32,
    pub num_columns: u16,
    pub num_params: u16,
    pub warning_count: Option<u16>,
}

pub enum MysqlComStmtPrepareResponse {
    Ok(MysqlComStmtPrepareOk),
    Error(MysqlErrPacketSummary),
}

pub fn parse_com_stmt_prepare_response(
    payload: &[u8],
) -> Result<Option<MysqlComStmtPrepareResponse>, MysqlComStmtPrepareResponseParseError>;

pub enum MysqlComStmtPrepareResponseParseError {
    IncompletePayload { field: &'static str, needed: usize, available: usize },
    ErrPacket { source: MysqlErrPacketParseError },
}
```

`MysqlConnectionState` exposes only the latest MySQL-local prepare outcome in this layer:

```rust
pub struct MysqlStatementPrepareOutcome {
    pub command: MysqlClientCommand,
    pub response_sequence_id: u8,
    pub response: MysqlStatementPrepareResponseState,
}

pub enum MysqlStatementPrepareResponseState {
    Prepared {
        statement_id: u32,
        num_columns: u16,
        num_params: u16,
        warning_count: Option<u16>,
    },
    Failed {
        error: MysqlErrPacketSummary,
    },
}

impl MysqlConnectionState {
    pub fn last_statement_prepare_outcome(&self) -> Option<&MysqlStatementPrepareOutcome>;
}
```

### 3. Contracts

- Backend prepare response observation runs only when phase is `Authenticated` and `pending_statement_prepare` exists.
- `observe_backend_bytes` always increments observed backend bytes and returns `ProtocolObservation::new(bytes.len(), 0)` for prepare response consumption.
- Successful prepare OK payloads start with `0x00`.
- Prepare OK parsing extracts little-endian `statement_id`, `num_columns`, and `num_params`.
- Prepare OK parsing consumes the reserved filler byte.
- Prepare OK parsing exposes `warning_count` when at least two warning-count bytes are present.
- Prepare OK payloads with exactly one trailing warning-count byte return `IncompletePayload { field: "warning_count" }`.
- Failed prepare responses use the existing MySQL ERR packet parser and are stored as a failed prepare outcome.
- Valid prepare OK and valid ERR responses consume `pending_statement_prepare`.
- Malformed, incomplete, unsupported, or unrecognized prepare responses are non-fatal and leave `pending_statement_prepare` intact.
- Valid prepare response consumption stores `last_statement_prepare_outcome` with the original prepare command and backend response sequence ID.
- Prepared statement map insertion belongs to the per-connection prepared statement state contract.
- Prepare response observation emits zero SQL events and does not call query timing logic for consumed prepare responses.
- Parsed prepare outcome is MySQL-specific and must not be projected into `SqlEvent`, `ConnectionInfo`, storage, API, WebSocket, or plugin contracts in this layer.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| Empty prepare response payload | Return `IncompletePayload { field: "header" }` |
| Payload starts with `0x00` and has all required prepare OK fields | Return `Some(MysqlComStmtPrepareResponse::Ok(...))` |
| Prepare OK is missing `statement_id`, `num_columns`, `num_params`, or filler | Return `IncompletePayload` for the missing field |
| Prepare OK has one warning-count byte | Return `IncompletePayload { field: "warning_count" }` |
| Payload starts with `0xff` and has a valid ERR packet | Return `Some(MysqlComStmtPrepareResponse::Error(...))` |
| Payload starts with `0xff` but ERR packet is malformed | Return `ErrPacket { source }` |
| Payload starts with another byte | Return `Ok(None)` |
| Backend prepare OK with pending prepare | Clear pending prepare, store prepared outcome, emit zero events |
| Backend prepare ERR with pending prepare | Clear pending prepare, store failed outcome, emit zero events |
| Backend prepare response without pending prepare | Emit zero events and do not update outcome |
| Malformed prepare response with pending prepare | Keep pending prepare and do not update outcome |
| Existing `COM_QUERY` backend OK/ERR flow | Keep current query event behavior unchanged |

### 5. Good/Base/Bad Cases

Good:

- Parser tests cover successful prepare OK, ERR, unrecognized payloads, and incomplete fields.
- Adapter tests first send a valid `COM_STMT_PREPARE`, then observe backend prepare OK or ERR.
- The backend packet envelope sequence ID is retained only in MySQL-specific prepare outcome metadata.

Base:

- Later statement-map work can consume `last_statement_prepare_outcome` to create a connection-local `statement_id -> template SQL` mapping.
- Later parameter and column definition parsing can use `num_params` and `num_columns` to know how many definition packets to expect.

Bad:

- Building the prepared statement map in the response parser task.
- Parsing parameter definition or column definition packets in the first response packet parser.
- Emitting a protocol-neutral SQL event from prepare response observation alone.
- Logging raw template SQL, packet payloads, or database error text from parser or adapter code.
- Adding MySQL prepared-statement fields directly to shared core models.

### 6. Tests Required

For MySQL `COM_STMT_PREPARE` response changes:

- Parser test for valid prepare OK statement ID.
- Parser test for valid prepare OK parameter and column counts.
- Parser test for valid prepare ERR response.
- Parser tests for incomplete prepare OK fields and malformed ERR packet.
- Adapter test proving prepare OK consumes pending prepare and stores prepared outcome.
- Adapter test proving prepare ERR consumes pending prepare and stores failed outcome.
- Adapter test proving backend prepare response without pending prepare is non-fatal.
- Adapter test proving malformed prepare response keeps pending prepare.
- Regression test coverage that existing `COM_QUERY` response behavior remains unchanged.
- Run `cargo fmt --check`.
- Run `cargo test -p sql-lens-protocol-mysql`.
- Run `cargo test --workspace`.
- Run `cargo clippy --workspace --all-targets -- -D warnings`.

### 7. Wrong vs Correct

#### Wrong

```rust
pub struct SqlEvent {
    pub mysql_statement_id: u32,
    pub mysql_param_count: u16,
}
```

#### Correct

```rust
pub struct MysqlStatementPrepareOutcome {
    pub command: MysqlClientCommand,
    pub response_sequence_id: u8,
    pub response: MysqlStatementPrepareResponseState,
}
```

## Scenario: MySQL Prepared Statement State Contracts

### 1. Scope / Trigger

- Trigger: `sql-lens-protocol-mysql` needs to retain successful prepared statement templates for later `COM_STMT_EXECUTE` parsing.
- This layer stores MySQL statement ID mappings inside a single `MysqlConnectionState`.
- It must not add a shared protocol close hook, parse execute packets, emit SQL events, expose storage/API/UI/plugin contracts, or update protocol-neutral core models.

### 2. Signatures

Prepared statement state lives in `crates/sql-lens-protocol-mysql/src/lib.rs`:

```rust
pub struct MysqlPreparedStatement {
    pub statement_id: u32,
    pub template_sql: String,
    pub num_columns: u16,
    pub num_params: u16,
    pub warning_count: Option<u16>,
}

impl MysqlConnectionState {
    pub fn prepared_statement(&self, statement_id: u32) -> Option<&MysqlPreparedStatement>;
    pub fn prepared_statement_count(&self) -> usize;
}
```

Implementation state uses a standard-library map owned by `MysqlConnectionState`; do not expose mutable map access.

### 3. Contracts

- A new `MysqlConnectionState` starts with zero prepared statements.
- Successful `COM_STMT_PREPARE` OK response inserts or replaces a prepared statement mapping in the current connection state.
- The map key is the server-assigned MySQL statement ID.
- The mapped value stores the statement ID, SQL template, parameter count, column count, and optional warning count.
- The SQL template comes from the original pending prepare command.
- Failed prepare responses update failed outcome state but do not insert into the prepared statement map.
- Reusing the same statement ID in one connection replaces the previous mapping with the latest successful prepare metadata.
- A separate `MysqlConnectionState` must not observe mappings from another connection state.
- Connection close cleanup is satisfied by per-connection ownership: when the connection state is dropped, its prepared statement map is dropped.
- `COM_STMT_CLOSE` removes a mapping from the current connection state when the close command is observed.
- Explicit shared close hooks and `COM_STMT_RESET` cleanup belong to later tasks.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| New connection state | `prepared_statement_count() == 0` |
| Successful prepare OK | Insert mapping keyed by `statement_id` |
| Mapping is read by statement ID | Return stored template and prepare metadata |
| Prepare ERR | Do not insert a mapping |
| Same statement ID prepared again in same connection | Replace existing mapping |
| Same statement ID exists in another connection | No cross-connection visibility |
| Connection state is dropped | Map is dropped with state ownership |
| Known statement ID is closed with `COM_STMT_CLOSE` | Remove that mapping |

### 5. Good/Base/Bad Cases

Good:

- Adapter tests drive insertion through real `COM_STMT_PREPARE` and prepare OK packets.
- Failed prepare tests assert the map remains empty.
- Cross-connection tests create two `MysqlConnectionState` instances and prove isolation.

Base:

- Later `COM_STMT_EXECUTE` parsing can call `prepared_statement(statement_id)` to find the template.
- Later `COM_STMT_RESET` can reset mappings or statement state when that command is implemented.

Bad:

- Storing prepared statements in a static/global map.
- Adding statement mappings to `SqlEvent`, `ConnectionInfo`, storage, API, WebSocket, or frontend schemas in this task.
- Adding a shared protocol close hook just to clear this local map.
- Logging raw SQL templates or packet payloads.

### 6. Tests Required

For MySQL prepared statement state changes:

- New state starts empty.
- Successful prepare OK inserts a mapping.
- Mapping includes statement ID, SQL template, parameter count, column count, and optional warning count.
- Prepare ERR does not insert a mapping.
- Same statement ID replacement updates the mapping.
- Separate connection states do not share mappings.
- Existing query and prepare response tests remain green.
- Run `cargo fmt --check`.
- Run `cargo test -p sql-lens-protocol-mysql`.
- Run `cargo test --workspace`.
- Run `cargo clippy --workspace --all-targets -- -D warnings`.

### 7. Wrong vs Correct

#### Wrong

```rust
static MYSQL_PREPARED_STATEMENTS: Mutex<HashMap<u32, String>> = ...;
```

#### Correct

```rust
pub struct MysqlConnectionState {
    prepared_statements: BTreeMap<u32, MysqlPreparedStatement>,
}
```

## Scenario: MySQL COM_STMT_EXECUTE Envelope Contracts

### 1. Scope / Trigger

- Trigger: `sql-lens-protocol-mysql` parses client-to-backend `COM_STMT_EXECUTE` command packets after authentication.
- This layer records the execute envelope and links a statement ID to connection-local prepared statement metadata when available.
- It must not decode NULL bitmap bytes, parameter types, parameter values, expanded SQL, SQL events, storage/API/UI contracts, or protocol-neutral core models.

### 2. Signatures

Public execute command parser types live in `crates/sql-lens-protocol-mysql/src/command.rs` and are re-exported from the crate root:

```rust
pub const MYSQL_COM_STMT_EXECUTE: u8 = 0x17;

pub enum MysqlCommandKind {
    Query,
    StatementPrepare,
    StatementExecute,
}

pub struct MysqlComStmtExecute {
    pub statement_id: u32,
    pub flags: u8,
    pub iteration_count: u32,
    pub has_parameter_payload: bool,
}

pub enum MysqlParsedClientCommand {
    Query(MysqlComQuery),
    StatementPrepare(MysqlComStmtPrepare),
    StatementExecute(MysqlComStmtExecute),
}
```

`MysqlConnectionState` exposes the latest MySQL-local execute envelope:

```rust
pub struct MysqlStatementExecuteEnvelope {
    pub command: MysqlClientCommand,
    pub statement_id: u32,
    pub flags: u8,
    pub iteration_count: u32,
    pub has_parameter_payload: bool,
    pub statement: Option<MysqlPreparedStatement>,
}

impl MysqlConnectionState {
    pub fn last_statement_execute_envelope(&self) -> Option<&MysqlStatementExecuteEnvelope>;
}
```

### 3. Contracts

- Client command observation happens only when phase is `Authenticated`.
- A payload starting with `MYSQL_COM_STMT_EXECUTE` parses the fixed command envelope after the command byte.
- The execute envelope extracts little-endian `statement_id`, one-byte `flags`, and little-endian `iteration_count`.
- `has_parameter_payload` is `true` when any bytes remain after the fixed execute envelope.
- Unknown statement IDs are represented non-fatally as an execute envelope with `statement: None`.
- Known statement IDs clone the current connection-local `MysqlPreparedStatement` into the execute envelope.
- `MysqlClientCommand.sql` stores the prepared statement template SQL when the statement ID is known and an empty string when unknown.
- Malformed or incomplete execute packets are non-fatal at adapter level and must not update `last_client_command` or `last_statement_execute_envelope`.
- Execute parsing emits zero SQL events and must not call timing-only logic intended for `COM_QUERY`.
- Parsed execute state is MySQL-specific and must not be projected into `SqlEvent`, `ConnectionInfo`, storage, API, WebSocket, or plugin contracts in this layer.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| Payload starts with `0x17` and has all envelope fields | Return `Some(MysqlParsedClientCommand::StatementExecute(...))` |
| Execute payload is missing `statement_id` | Return `IncompletePayload { field: "statement_id" }` |
| Execute payload is missing `flags` | Return `IncompletePayload { field: "flags" }` |
| Execute payload is missing `iteration_count` | Return `IncompletePayload { field: "iteration_count" }` |
| Execute payload has bytes after iteration count | Set `has_parameter_payload = true` |
| `COM_STMT_EXECUTE` before `Authenticated` | Count bytes only; do not update command or execute state; emit zero events |
| Known statement ID after `Authenticated` | Store command and execute envelope with `statement: Some(...)`; emit zero events |
| Unknown statement ID after `Authenticated` | Store command and execute envelope with `statement: None`; emit zero events |
| Malformed execute command after `Authenticated` | Keep phase `Authenticated`; do not update command or execute state; emit zero events |

### 5. Good/Base/Bad Cases

Good:

- Parser tests cover valid statement ID, flags, iteration count, trailing parameter payload bytes, and incomplete envelope fields.
- Adapter tests drive known statement linking through real `COM_STMT_PREPARE` and prepare OK packets before execute.
- Unknown statement ID tests assert a successful envelope with `statement: None`.

Base:

- Later parameter tasks can use `statement.num_params` and `has_parameter_payload` to decode NULL bitmap, parameter type metadata, and parameter values.
- Later expanded SQL rendering can read the cloned statement template without changing the execute envelope contract.
- `COM_STMT_CLOSE` cleanup removes mappings before execute lookup; later
  `COM_STMT_RESET` cleanup can add reset-specific behavior.

Bad:

- Treating unknown statement IDs as `ProtocolAdapterError`.
- Decoding parameter values in the envelope parser task.
- Emitting a protocol-neutral SQL event from execute command observation alone.
- Logging raw SQL templates, parameter bytes, or packet payloads.
- Adding MySQL execute fields directly to shared core models.

### 6. Tests Required

For MySQL `COM_STMT_EXECUTE` envelope changes:

- Parser test for valid statement ID extraction.
- Parser test for flags extraction.
- Parser test for iteration count extraction.
- Parser test for trailing parameter payload detection.
- Parser tests for incomplete `statement_id`, `flags`, and `iteration_count`.
- Adapter test proving `COM_STMT_EXECUTE` before authentication does not update state.
- Adapter test proving known statement IDs link to prepared statement metadata.
- Adapter test proving unknown statement IDs stay non-fatal with `statement: None`.
- Adapter test proving malformed execute commands after authentication stay non-fatal.
- Regression test coverage that existing `COM_QUERY` and `COM_STMT_PREPARE` behavior remains unchanged.
- Run `cargo fmt --check`.
- Run `cargo test -p sql-lens-protocol-mysql`.
- Run `cargo test --workspace`.
- Run `cargo clippy --workspace --all-targets -- -D warnings`.

### 7. Wrong vs Correct

#### Wrong

```rust
return Err(ProtocolAdapterError::ObservationFailed("unknown statement id"));
```

#### Correct

```rust
MysqlStatementExecuteEnvelope {
    statement_id,
    statement: prepared_statement(statement_id).cloned(),
    // remaining envelope fields...
}
```

## Scenario: MySQL COM_STMT_CLOSE Cleanup Contracts

### 1. Scope / Trigger

- Trigger: `sql-lens-protocol-mysql` observes a client-to-backend
  `COM_STMT_CLOSE` command after authentication.
- This layer removes the closed statement ID from the current
  `MysqlConnectionState` prepared statement map.
- `COM_STMT_CLOSE` has no backend OK/ERR response, so cleanup happens during
  client command observation.
- It must not emit SQL events, touch storage/API/WebSocket/UI contracts, modify
  forwarded traffic, or add a shared protocol close hook.

### 2. Signatures

Close command parser types live in `crates/sql-lens-protocol-mysql/src/command.rs`
and are re-exported from the crate root:

```rust
pub const MYSQL_COM_STMT_CLOSE: u8 = 0x19;

pub enum MysqlCommandKind {
    Query,
    StatementPrepare,
    StatementExecute,
    StatementClose,
}

pub struct MysqlComStmtClose {
    pub statement_id: u32,
}

pub enum MysqlParsedClientCommand {
    Query(MysqlComQuery),
    StatementPrepare(MysqlComStmtPrepare),
    StatementExecute(MysqlComStmtExecute),
    StatementClose(MysqlComStmtClose),
}
```

### 3. Contracts

- Client command observation happens only when phase is `Authenticated`.
- A payload starting with `MYSQL_COM_STMT_CLOSE` parses the fixed close payload
  after the command byte.
- The close payload extracts little-endian `statement_id`.
- Known statement IDs are removed from the current connection-local prepared
  statement map immediately.
- Unknown statement IDs are harmless: do not panic and do not alter existing
  mappings.
- Malformed or incomplete close packets are non-fatal at adapter level and must
  not update `last_client_command` or prepared statement state.
- Successful close observation stores `last_client_command.kind =
  MysqlCommandKind::StatementClose` with empty SQL.
- Close cleanup emits zero SQL events and must not call timing-only logic
  intended for `COM_QUERY`.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| Payload starts with `0x19` and has four statement ID bytes | Return `Some(MysqlParsedClientCommand::StatementClose(...))` |
| Close payload is missing `statement_id` | Return `IncompletePayload { field: "statement_id" }` |
| `COM_STMT_CLOSE` before `Authenticated` | Count bytes only; do not update statement state; emit zero events |
| Known statement ID after `Authenticated` | Remove mapping and store close command; emit zero events |
| Unknown statement ID after `Authenticated` | Keep existing mappings and store close command; emit zero events |
| Malformed close command after `Authenticated` | Keep phase `Authenticated`; do not update command or statement state; emit zero events |

### 5. Good/Base/Bad Cases

Good:

- Parser tests cover statement ID extraction and incomplete statement ID
  payloads.
- Adapter tests prepare a statement, close it, and assert the mapping is gone.
- Unknown close tests assert other mappings remain intact.

Base:

- Later event-emission work can decide whether statement close should become a
  protocol-neutral `SqlEventKind::StatementClose`.
- Later `COM_STMT_RESET` work can add reset semantics without changing close
  parsing.

Bad:

- Waiting for a backend response before removing the mapping.
- Treating unknown statement IDs as `ProtocolAdapterError`.
- Clearing all prepared statements for one close command.
- Emitting storage/API/WebSocket-visible close events in this task.
- Logging raw SQL templates or packet payloads.

### 6. Tests Required

For MySQL `COM_STMT_CLOSE` cleanup changes:

- Parser test for valid statement ID extraction.
- Parser test for missing statement ID.
- Adapter test proving a known statement ID is removed after close.
- Adapter test proving an unknown close leaves existing mappings intact.
- Adapter test proving malformed close packets do not mutate state.
- Regression coverage that existing prepare and execute behavior remains
  unchanged.
- Run `cargo fmt --check`.
- Run `cargo test -p sql-lens-protocol-mysql`.
- Run `cargo test --workspace`.
- Run `cargo clippy --workspace --all-targets -- -D warnings`.

## Scenario: MySQL COM_STMT_EXECUTE NULL Bitmap Contracts

### 1. Scope / Trigger

- Trigger: `sql-lens-protocol-mysql` decodes the NULL bitmap inside client-to-backend `COM_STMT_EXECUTE` parameter payloads.
- This layer records zero-based NULL parameter indexes in MySQL-local execute envelope state when prepared statement metadata is known.
- It must not decode `new_params_bind_flag`, parameter types, parameter values, expanded SQL, redaction, storage/API/UI contracts, or protocol-neutral core models.

### 2. Signatures

Public NULL bitmap parser types live in `crates/sql-lens-protocol-mysql/src/execute.rs` and are re-exported from the crate root:

```rust
pub struct MysqlNullBitmap {
    pub null_parameter_indexes: Vec<usize>,
    pub bytes_consumed: usize,
}

pub fn decode_null_bitmap(
    parameter_payload: &[u8],
    parameter_count: u16,
) -> Result<MysqlNullBitmap, MysqlExecuteParseError>;

pub enum MysqlExecuteParseError {
    IncompletePayload {
        field: &'static str,
        needed: usize,
        available: usize,
    },
}
```

`MysqlConnectionState` exposes decoded NULL indexes only through MySQL-local execute envelope state:

```rust
pub struct MysqlStatementExecuteEnvelope {
    pub command: MysqlClientCommand,
    pub statement_id: u32,
    pub flags: u8,
    pub iteration_count: u32,
    pub has_parameter_payload: bool,
    pub statement: Option<MysqlPreparedStatement>,
    pub null_parameter_indexes: Vec<usize>,
}
```

### 3. Contracts

- NULL bitmap length is derived from prepared statement parameter count: `(parameter_count + 7) / 8`.
- Bits map to zero-based parameter indexes with execute bitmap bit offset `0`.
- Inside each byte, bit `0` maps to the lowest parameter index for that byte.
- Parser ignores padding bits beyond `parameter_count`.
- `bytes_consumed` is the computed bitmap length, including `0` for zero-parameter statements.
- Truncated bitmap bytes return `MysqlExecuteParseError::IncompletePayload { field: "null_bitmap" }`.
- Adapter-level bitmap parsing runs only when the execute statement ID is known and has connection-local `MysqlPreparedStatement` metadata.
- Unknown statement IDs remain non-fatal and store `null_parameter_indexes = Vec::new()`.
- Known statements with zero parameters store an empty NULL index list.
- Known statements with truncated NULL bitmap bytes are non-fatal at adapter level and must not update `last_client_command` or `last_statement_execute_envelope`.
- Raw parameter payload bytes must not be stored in connection state.
- NULL bitmap decoding emits zero SQL events and must not call timing-only logic intended for `COM_QUERY`.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| `parameter_count = 0` and payload empty | Return empty indexes with `bytes_consumed = 0` |
| Bitmap has mixed NULL and non-NULL bits | Return zero-based indexes for set bits within `parameter_count` |
| Bitmap has no NULL bits | Return empty indexes with computed `bytes_consumed` |
| Bitmap has set padding bits beyond `parameter_count` | Ignore padding bits |
| Available bytes are shorter than computed bitmap length | Return `IncompletePayload { field: "null_bitmap" }` |
| Known statement ID after `Authenticated` | Decode bitmap and store indexes on execute envelope; emit zero events |
| Unknown statement ID after `Authenticated` | Store execute envelope with empty NULL indexes; emit zero events |
| Truncated known-statement bitmap after `Authenticated` | Keep phase `Authenticated`; do not update command or execute state; emit zero events |

### 5. Good/Base/Bad Cases

Good:

- Parser tests cover mixed NULL bits, all non-NULL parameters, zero parameters, ignored padding bits, and truncated bitmap errors.
- Adapter tests drive known-statement bitmap decoding through real `COM_STMT_PREPARE`, prepare OK, and execute packets.
- Unknown statement ID tests assert empty NULL index state without trying to infer parameter count.

Base:

- Later parameter type decoding can begin at `bytes_consumed` after the NULL bitmap.
- Later value decoding can use `null_parameter_indexes` to skip values for NULL parameters.
- Later expanded SQL rendering can use NULL indexes without changing this bitmap contract.

Bad:

- Treating a truncated bitmap as a `ProtocolAdapterError`.
- Decoding parameter types or values in the NULL bitmap parser task.
- Storing raw parameter payload bytes in `MysqlConnectionState`.
- Adding MySQL NULL bitmap fields directly to shared core models.
- Logging parameter payload bytes or raw SQL templates.

### 6. Tests Required

For MySQL `COM_STMT_EXECUTE` NULL bitmap changes:

- Parser test for mixed NULL and non-NULL parameters.
- Parser test for all non-NULL parameters.
- Parser test for zero parameters.
- Parser test proving padding bits beyond parameter count are ignored.
- Parser test for truncated bitmap bytes.
- Adapter test proving known statement IDs store decoded NULL indexes.
- Adapter test proving unknown statement IDs remain non-fatal with empty NULL indexes.
- Adapter test proving truncated known-statement bitmaps stay non-fatal and do not update execute state.
- Regression test coverage that existing execute envelope, prepare, and query behavior remains unchanged.
- Run `cargo fmt --check`.
- Run `cargo test -p sql-lens-protocol-mysql`.
- Run `cargo test --workspace`.
- Run `cargo clippy --workspace --all-targets -- -D warnings`.

### 7. Wrong vs Correct

#### Wrong

```rust
pub struct MysqlStatementExecuteEnvelope {
    pub raw_parameter_payload: Vec<u8>,
}
```

#### Correct

```rust
pub struct MysqlStatementExecuteEnvelope {
    pub null_parameter_indexes: Vec<usize>,
}
```

## Scenario: MySQL COM_STMT_EXECUTE Numeric Parameter Contracts

### 1. Scope / Trigger

- Trigger: `sql-lens-protocol-mysql` decodes common numeric prepared statement parameters inside client-to-backend `COM_STMT_EXECUTE` parameter payloads.
- This layer records MySQL-local decoded numeric parameters when prepared statement metadata is known and current-packet parameter type metadata is present.
- It must not implement cross-execute parameter type caching, string/binary/date/time/decimal/JSON decoding, expanded SQL rendering, redaction, storage/API/UI contracts, or protocol-neutral event emission.

### 2. Signatures

Public numeric parameter parser types live in `crates/sql-lens-protocol-mysql/src/execute.rs` and are re-exported from the crate root:

```rust
pub struct MysqlParameterType {
    pub type_code: u8,
    pub unsigned: bool,
}

pub struct MysqlDecodedParameter {
    pub index: u16,
    pub value: sql_lens_core::SqlParameterValue,
}

pub struct MysqlDecodedParameters {
    pub parameters: Vec<MysqlDecodedParameter>,
    pub bytes_consumed: usize,
}

pub fn decode_numeric_parameters(
    parameter_payload_after_null_bitmap: &[u8],
    parameter_count: u16,
    null_parameter_indexes: &[usize],
) -> Result<Option<MysqlDecodedParameters>, MysqlExecuteParseError>;

pub fn decode_parameters(
    parameter_payload_after_null_bitmap: &[u8],
    parameter_count: u16,
    null_parameter_indexes: &[usize],
) -> Result<Option<MysqlDecodedParameters>, MysqlExecuteParseError>;
```

`MysqlConnectionState` exposes decoded numeric parameters only through MySQL-local execute envelope state:

```rust
pub struct MysqlStatementExecuteEnvelope {
    pub null_parameter_indexes: Vec<usize>,
    pub parameters: Vec<MysqlDecodedParameter>,
}
```

### 3. Contracts

- Decode numeric values only after NULL bitmap decoding has identified NULL parameter indexes.
- `parameter_payload_after_null_bitmap` begins with `new_params_bind_flag` when `parameter_count > 0`.
- `new_params_bind_flag = 1` means current-packet parameter type metadata is present and can be decoded.
- `new_params_bind_flag = 0` is non-fatal unsupported in this layer; return `Ok(None)` and do not decode values until a later type-cache task exists.
- Parameter type metadata is two bytes per parameter: type code plus flag byte.
- The unsigned marker is the high bit of the flag byte (`0x80`).
- Supported numeric type codes are `TINY`, `SHORT`, `LONG`, `LONGLONG`, `INT24`, `FLOAT`, and `DOUBLE`.
- `TINY`, `SHORT`, `LONG`, `LONGLONG`, and `INT24` decode to `SqlParameterValue::Integer` or `SqlParameterValue::Unsigned` according to the unsigned flag.
- `FLOAT` and `DOUBLE` decode to `SqlParameterValue::Float`.
- NULL parameter indexes produce `SqlParameterValue::Null` and do not consume value bytes.
- Unsupported type codes are non-fatal unsupported; return `Ok(None)` and do not expose partial decoded parameter state.
- Truncated bind flag, parameter type metadata, or numeric value bytes return `MysqlExecuteParseError::IncompletePayload`.
- Adapter-level numeric parsing runs only when the execute statement ID is known and has connection-local `MysqlPreparedStatement` metadata.
- Unknown statement IDs remain non-fatal and store `parameters = Vec::new()`.
- Malformed numeric payloads are non-fatal at adapter level and must not update `last_client_command` or `last_statement_execute_envelope`.
- Raw parameter payload bytes must not be stored in connection state.
- Numeric parameter decoding emits zero SQL events and must not call timing-only logic intended for `COM_QUERY`.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| `parameter_count = 0` and payload empty | Return empty decoded parameters with `bytes_consumed = 0` |
| `new_params_bind_flag = 1` with supported signed integer types | Decode as `SqlParameterValue::Integer` |
| `new_params_bind_flag = 1` with supported unsigned integer types | Decode as `SqlParameterValue::Unsigned` |
| `new_params_bind_flag = 1` with `FLOAT` or `DOUBLE` | Decode as `SqlParameterValue::Float` |
| Parameter index is listed in NULL bitmap | Decode as `SqlParameterValue::Null` and consume no value bytes |
| `new_params_bind_flag = 0` | Return `Ok(None)`; adapter may still store execute envelope and NULL bitmap state |
| Unsupported type code | Return `Ok(None)` without exposing partial decoded parameter state |
| Missing bind flag | Return `IncompletePayload { field: "new_params_bind_flag" }` |
| Truncated type metadata | Return `IncompletePayload { field: "parameter_types" }` |
| Truncated numeric value | Return `IncompletePayload { field: "parameter_value" }` |
| Known statement ID with valid numeric payload | Store decoded numeric parameters on execute envelope; emit zero events |
| Unknown statement ID | Store empty numeric parameters; emit zero events |
| Malformed numeric payload after `Authenticated` | Keep phase `Authenticated`; do not update command or execute state; emit zero events |

### 5. Good/Base/Bad Cases

Good:

- Parser tests cover signed integers, unsigned integers, `FLOAT`, `DOUBLE`, NULL numeric parameters, missing bind flag, truncated type metadata, and truncated values.
- Adapter tests drive numeric decoding through real `COM_STMT_PREPARE`, prepare OK, and execute packets.
- `new_params_bind_flag = 0` tests assert non-fatal behavior without cross-execute type caching.

Base:

- String/binary/date/time tasks can add supported type-code branches without changing the NULL bitmap or bind-flag contract.
- Later expanded SQL rendering can consume MySQL-local decoded parameters or convert them into core `SqlParameter` values.
- Later per-statement type caching can add support for `new_params_bind_flag = 0`.

Bad:

- Attempting to decode values when `new_params_bind_flag = 0` without a type cache.
- Treating unsupported type codes as adapter errors.
- Storing raw parameter payload bytes in `MysqlConnectionState`.
- Emitting `SqlEvent` from numeric decoding alone.
- Adding decoded MySQL parameter details directly to shared core models before event emission needs them.
- Logging parameter payload bytes, raw SQL templates, or decoded sensitive values.

### 6. Tests Required

For MySQL `COM_STMT_EXECUTE` numeric parameter changes:

- Parser test for signed integer representative values.
- Parser test for unsigned integer representative values.
- Parser test for `FLOAT` and `DOUBLE` values.
- Parser test proving NULL parameters do not consume value bytes.
- Parser test for `new_params_bind_flag = 0` returning unsupported `None`.
- Parser test for unsupported type code returning unsupported `None`.
- Parser tests for missing bind flag, truncated type metadata, and truncated numeric value bytes.
- Adapter test proving known statement IDs store decoded parameters.
- Adapter test proving malformed numeric payloads stay non-fatal and do not update execute state.
- Regression test coverage that existing execute envelope, NULL bitmap, prepare, and query behavior remains unchanged.
- Run `cargo fmt --check`.
- Run `cargo test -p sql-lens-protocol-mysql`.
- Run `cargo test --workspace`.
- Run `cargo clippy --workspace --all-targets -- -D warnings`.

### 7. Wrong vs Correct

#### Wrong

```rust
if new_params_bind_flag == 0 {
    decode_using_last_seen_types_without_storing_them();
}
```

#### Correct

```rust
if new_params_bind_flag != 1 {
    return Ok(None);
}
```

## Scenario: MySQL COM_STMT_EXECUTE String and Binary Parameter Contracts

### 1. Scope / Trigger

- Trigger: `sql-lens-protocol-mysql` decodes common string-like and binary prepared statement parameters inside client-to-backend `COM_STMT_EXECUTE` parameter payloads.
- This layer records MySQL-local decoded parameter values when prepared statement metadata is known and current-packet parameter type metadata is present.
- It must not implement cross-execute parameter type caching, date/time/decimal/JSON decoding, expanded SQL rendering, redaction, storage/API/UI contracts, or protocol-neutral event emission.

### 2. Signatures

Common parameter decoding lives in `crates/sql-lens-protocol-mysql/src/execute.rs` and is re-exported from the crate root:

```rust
pub fn decode_parameters(
    parameter_payload_after_null_bitmap: &[u8],
    parameter_count: u16,
    null_parameter_indexes: &[usize],
) -> Result<Option<MysqlDecodedParameters>, MysqlExecuteParseError>;
```

`MysqlStatementExecuteEnvelope` stores decoded values in a protocol-local field:

```rust
pub struct MysqlStatementExecuteEnvelope {
    pub null_parameter_indexes: Vec<usize>,
    pub parameters: Vec<MysqlDecodedParameter>,
}
```

### 3. Contracts

- Decode string and binary values only after NULL bitmap decoding has identified NULL parameter indexes.
- `new_params_bind_flag = 1` means current-packet parameter type metadata is present and can be decoded.
- `new_params_bind_flag = 0` remains non-fatal unsupported until a later type-cache task exists.
- Text type codes are `VARCHAR`, `VAR_STRING`, `STRING`, `ENUM`, and `SET`.
- Binary summary type codes are `TINY_BLOB`, `MEDIUM_BLOB`, `LONG_BLOB`, `BLOB`, `BIT`, and `GEOMETRY`.
- Text and binary values are MySQL length-encoded byte strings.
- Text values decode to `SqlParameterValue::String` with UTF-8 lossy replacement for invalid byte sequences.
- Binary values decode to `SqlParameterValue::BinarySummary` with total byte length and a short lowercase hex prefix.
- Binary summaries must not contain full raw binary payloads when the value is longer than the summary prefix.
- NULL parameter indexes produce `SqlParameterValue::Null` and do not consume value bytes.
- Unsupported type codes are non-fatal unsupported; return `Ok(None)` and do not expose partial decoded parameter state.
- Truncated length prefixes or value bytes return `MysqlExecuteParseError::IncompletePayload`.
- Invalid length-encoded integer markers return `MysqlExecuteParseError::InvalidLengthEncodedInteger`.
- Adapter-level parameter parsing runs only when the execute statement ID is known and has connection-local `MysqlPreparedStatement` metadata.
- Unknown statement IDs remain non-fatal and store `parameters = Vec::new()`.
- Malformed parameter payloads are non-fatal at adapter level and must not update `last_client_command` or `last_statement_execute_envelope`.
- Raw parameter payload bytes must not be stored in connection state.
- Parameter decoding emits zero SQL events and must not call timing-only logic intended for `COM_QUERY`.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| Supported text value with valid UTF-8 | Decode as `SqlParameterValue::String` |
| Supported text value with invalid UTF-8 | Decode as `SqlParameterValue::String` with replacement characters |
| Supported binary value | Decode as `SqlParameterValue::BinarySummary` |
| Binary value exceeds summary prefix | Include `...` and do not include full raw payload |
| Mixed numeric, text, binary, and NULL values | Preserve parameter order in `parameters` |
| `new_params_bind_flag = 0` | Return `Ok(None)`; adapter may still store execute envelope and NULL bitmap state |
| Unsupported type code | Return `Ok(None)` without exposing partial decoded parameter state |
| Truncated length prefix | Return `IncompletePayload { field: "parameter_value" }` |
| Truncated text or binary value | Return `IncompletePayload { field: "parameter_value" }` |
| Known statement ID with valid string/binary payload | Store decoded parameters on execute envelope; emit zero events |
| Unknown statement ID | Store empty decoded parameters; emit zero events |
| Malformed payload after `Authenticated` | Keep phase `Authenticated`; do not update command or execute state; emit zero events |

### 5. Good/Base/Bad Cases

Good:

- Parser tests cover valid text, invalid UTF-8 text, binary summaries, mixed parameters, and truncated length-encoded payloads.
- Adapter tests drive text and binary decoding through real `COM_STMT_PREPARE`, prepare OK, and execute packets.
- BLOB-family values are summarized by default because charset metadata is not available in this layer.

Base:

- Later charset-aware decoding can replace UTF-8 lossy decoding once SQL Lens models enough MySQL metadata.
- Later date/time, decimal, and JSON tasks can add supported type-code branches to `decode_parameters`.
- Later expanded SQL rendering can consume MySQL-local decoded parameters or convert them into core `SqlParameter` values.

Bad:

- Storing raw binary parameter bytes in `MysqlConnectionState`.
- Logging raw parameter payload bytes, raw SQL templates, or decoded sensitive values.
- Treating invalid UTF-8 text as a panic or adapter failure.
- Decoding BLOB-family values as full strings without charset metadata.
- Emitting `SqlEvent` from parameter decoding alone.
- Adding decoded MySQL parameter details directly to shared core models before event emission needs them.

### 6. Tests Required

For MySQL `COM_STMT_EXECUTE` string and binary parameter changes:

- Parser test for supported text values.
- Parser test for invalid UTF-8 text replacement.
- Parser test for binary summaries with short and long values.
- Parser test for mixed numeric, text, binary, and NULL parameters.
- Parser tests for truncated length prefix and truncated value bytes.
- Adapter test proving known statement IDs store decoded text and binary summary parameters.
- Regression test coverage that existing numeric, NULL bitmap, execute envelope, prepare, and query behavior remains unchanged.
- Run `cargo fmt --check`.
- Run `cargo test -p sql-lens-protocol-mysql`.
- Run `cargo test --workspace`.
- Run `cargo clippy --workspace --all-targets -- -D warnings`.

### 7. Wrong vs Correct

#### Wrong

```rust
SqlParameterValue::String(String::from_utf8(raw_binary_blob).unwrap())
```

#### Correct

```rust
SqlParameterValue::BinarySummary(binary_summary(raw_binary_blob))
```

## Scenario: MySQL COM_STMT_EXECUTE Temporal Parameter Contracts

### 1. Scope / Trigger

- Trigger: `sql-lens-protocol-mysql` decodes MySQL binary protocol date and time prepared statement parameters inside client-to-backend `COM_STMT_EXECUTE` parameter payloads.
- This layer records MySQL-local decoded temporal parameter values when prepared statement metadata is known and current-packet parameter type metadata is present.
- It must not implement cross-execute parameter type caching, timezone conversion, strict calendar validation, expanded SQL rendering, redaction, storage/API/UI contracts, or protocol-neutral event emission.

### 2. Signatures

Temporal parameter decoding is part of the shared MySQL parameter decoder in `crates/sql-lens-protocol-mysql/src/execute.rs`:

```rust
pub fn decode_parameters(
    parameter_payload_after_null_bitmap: &[u8],
    parameter_count: u16,
    null_parameter_indexes: &[usize],
) -> Result<Option<MysqlDecodedParameters>, MysqlExecuteParseError>;
```

Temporal values use existing core parameter value variants:

```rust
SqlParameterValue::Date(String)
SqlParameterValue::Time(String)
SqlParameterValue::Timestamp(String)
```

### 3. Contracts

- Decode temporal values only after NULL bitmap decoding has identified NULL parameter indexes.
- `new_params_bind_flag = 1` means current-packet parameter type metadata is present and can be decoded.
- `new_params_bind_flag = 0` remains non-fatal unsupported until a later type-cache task exists.
- Supported temporal type codes are `DATE`, `NEWDATE`, `TIME`, `DATETIME`, and `TIMESTAMP`.
- Date and datetime values are MySQL length-prefixed binary values, not text.
- `DATE` and `NEWDATE` accept lengths `0` and `4`.
- `DATETIME` and `TIMESTAMP` accept lengths `0`, `4`, `7`, and `11`.
- `TIME` accepts lengths `0`, `8`, and `12`.
- `DATE` and `NEWDATE` decode to `SqlParameterValue::Date("YYYY-MM-DD")`.
- `DATETIME` and `TIMESTAMP` decode to `SqlParameterValue::Timestamp("YYYY-MM-DD HH:MM:SS")`.
- Temporal values with microsecond payloads append `.ffffff`.
- `TIME` decodes to `SqlParameterValue::Time("HH:MM:SS")` when days are zero.
- `TIME` decodes to `SqlParameterValue::Time("D HH:MM:SS")` when days are non-zero.
- Negative `TIME` values are prefixed with `-`.
- Zero-length date values decode as `0000-00-00`.
- Zero-length datetime and timestamp values decode as `0000-00-00 00:00:00`.
- Zero-length time values decode as `00:00:00`.
- Unsupported temporal lengths return `MysqlExecuteParseError::InvalidTemporalValueLength`.
- Truncated temporal payloads return `MysqlExecuteParseError::IncompletePayload`.
- The parser formats temporal strings but does not validate real calendar dates.
- Adapter-level temporal parsing runs only when the execute statement ID is known and has connection-local `MysqlPreparedStatement` metadata.
- Malformed temporal payloads are non-fatal at adapter level and must not update `last_client_command` or `last_statement_execute_envelope`.
- Raw parameter payload bytes must not be stored in connection state.
- Temporal parameter decoding emits zero SQL events and must not call timing-only logic intended for `COM_QUERY`.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| `DATE` or `NEWDATE` with length `4` | Decode as `SqlParameterValue::Date` |
| `DATE` or `NEWDATE` with length `0` | Decode as zero date |
| `TIME` with length `8` | Decode as `SqlParameterValue::Time` |
| `TIME` with length `12` | Decode as `SqlParameterValue::Time` with microseconds |
| Negative `TIME` | Prefix the formatted value with `-` |
| `DATETIME` or `TIMESTAMP` with length `4` | Decode as date plus `00:00:00` |
| `DATETIME` or `TIMESTAMP` with length `7` | Decode date and seconds precision time |
| `DATETIME` or `TIMESTAMP` with length `11` | Decode date and microsecond precision time |
| Unsupported temporal length | Return `InvalidTemporalValueLength` |
| Truncated temporal value | Return `IncompletePayload { field: "parameter_value" }` |
| Known statement ID with valid temporal payload | Store decoded parameters on execute envelope; emit zero events |
| Malformed payload after `Authenticated` | Keep phase `Authenticated`; do not update command or execute state; emit zero events |

### 5. Good/Base/Bad Cases

Good:

- Parser tests cover date, zero temporal values, time, negative time, microsecond time, datetime, timestamp, unsupported temporal length, and truncated temporal payload.
- Adapter tests drive temporal decoding through real `COM_STMT_PREPARE`, prepare OK, and execute packets.
- Zero dates remain representable as strings because MySQL can carry values strict date libraries reject.

Base:

- Later expanded SQL rendering can consume these MySQL-local decoded values.
- Later charset or field metadata tasks do not need to affect temporal decoding.
- Later type-cache work can add `new_params_bind_flag = 0` support without changing temporal value representation.

Bad:

- Applying timezone conversion in the packet decoder.
- Rejecting zero dates through strict calendar validation.
- Logging raw parameter payload bytes, raw SQL templates, or decoded sensitive values.
- Emitting `SqlEvent` from temporal decoding alone.
- Adding temporal MySQL details directly to shared core models before event emission needs them.

### 6. Tests Required

For MySQL `COM_STMT_EXECUTE` temporal parameter changes:

- Parser test for `DATE` and `NEWDATE`.
- Parser test for zero-length temporal values.
- Parser test for `TIME`, negative `TIME`, and microsecond `TIME`.
- Parser test for `DATETIME` and `TIMESTAMP`.
- Parser test for unsupported temporal length.
- Parser test for truncated temporal payload.
- Adapter test proving known statement IDs store decoded temporal parameters.
- Regression test coverage that existing numeric, string, binary, NULL bitmap, execute envelope, prepare, and query behavior remains unchanged.
- Run `cargo fmt --check`.
- Run `cargo test -p sql-lens-protocol-mysql`.
- Run `cargo test --workspace`.
- Run `cargo clippy --workspace --all-targets -- -D warnings`.

### 7. Wrong vs Correct

#### Wrong

```rust
parse_date_strictly_or_panic("0000-00-00")
```

#### Correct

```rust
SqlParameterValue::Date("0000-00-00".to_owned())
```

## Scenario: MySQL Prepared Statement Expanded SQL Rendering Contracts

### 1. Scope / Trigger

- Trigger: `sql-lens-protocol-mysql` renders a readable expanded SQL string for MySQL-compatible prepared statement executions after `COM_STMT_EXECUTE` parameter decoding.
- This layer is MySQL-local until prepared statement event emission and redaction are implemented.
- It must not modify forwarded traffic, emit prepared statement `SqlEvent`s, persist expanded SQL, broadcast expanded SQL, normalize SQL, fingerprint SQL, or apply redaction policy.

### 2. Signatures

Expanded SQL rendering lives in `crates/sql-lens-protocol-mysql/src/execute.rs` and is re-exported from the crate root:

```rust
pub fn render_expanded_sql(
    template_sql: &str,
    parameters: &[MysqlDecodedParameter],
) -> Result<String, MysqlExpandedSqlRenderError>;

pub enum MysqlExpandedSqlRenderError {
    MissingParameter {
        placeholder_index: usize,
        parameter_count: usize,
    },
    ExtraParameters {
        placeholder_count: usize,
        parameter_count: usize,
    },
}
```

`MysqlStatementExecuteEnvelope` stores the MySQL-local rendered result:

```rust
pub struct MysqlStatementExecuteEnvelope {
    pub parameters: Vec<MysqlDecodedParameter>,
    pub expanded_sql: Option<String>,
}
```

### 3. Contracts

- Rendered SQL is display-oriented debugging output, not an executable replay contract.
- Rendering replaces `?` placeholders only in normal SQL context.
- Placeholder scanning must skip `?` inside single-quoted strings, double-quoted strings, backtick-quoted identifiers, `-- ` line comments, `#` line comments, and `/* ... */` block comments.
- Single-quoted strings support doubled quote escapes such as `''`.
- Single-quoted and double-quoted strings support backslash escaping while scanning.
- `SqlParameterValue::Null` renders as `NULL`.
- `Integer`, `Unsigned`, and `Float` render without quotes.
- `Boolean` renders as `TRUE` or `FALSE`.
- `String`, `Date`, `Time`, `Timestamp`, `Json`, `BinarySummary`, and `Unsupported` render as single-quoted display literals.
- Single quotes inside display literals are escaped by doubling them.
- Binary values render only the existing binary summary string; raw binary bytes must not be reconstructed or stored.
- If there are more placeholders than decoded parameters, return `MysqlExpandedSqlRenderError::MissingParameter`.
- If there are more decoded parameters than placeholders, return `MysqlExpandedSqlRenderError::ExtraParameters`.
- Known statement IDs with complete decoded parameters and successful rendering store `expanded_sql = Some(...)` on the MySQL-local execute envelope.
- Unknown statement IDs store `expanded_sql = None`.
- Unsupported parameter decoding stores `expanded_sql = None`.
- Render mismatch is non-fatal at adapter level and must not update `last_client_command` or `last_statement_execute_envelope` for malformed known-statement payloads.
- Rendering emits zero SQL events and must not call timing-only logic intended for `COM_QUERY`.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| String contains `'` | Escape as doubled single quote |
| Parameter is `NULL` | Render `NULL` |
| Parameter is numeric | Render without quotes |
| Parameter is boolean | Render `TRUE` or `FALSE` |
| Parameter is date/time/timestamp/json/binary summary/unsupported | Render as a quoted display literal |
| `?` inside quoted string, quoted identifier, or comment | Leave unchanged |
| Placeholder lacks a parameter | Return `MissingParameter` |
| Extra decoded parameter remains after scanning | Return `ExtraParameters` |
| Known statement ID with complete parameters | Store expanded SQL on execute envelope; emit zero events |
| Unknown statement ID | Store `expanded_sql = None`; emit zero events |
| Rendering would fail after `Authenticated` | Keep phase `Authenticated`; do not update command or execute state; emit zero events |

### 5. Good/Base/Bad Cases

Good:

- Parser tests cover strings with quotes, `NULL`, numeric, boolean, date/time/timestamp, JSON, binary summaries, skipped placeholders, and count mismatches.
- Adapter tests drive rendering through real `COM_STMT_PREPARE`, prepare OK, and execute packets.
- Rendered SQL stays MySQL-local until redaction and event emission tasks define exposure rules.

Base:

- Later prepared statement event emission can copy this rendered string into `SqlEvent.expanded_sql` after redaction policy is applied.
- Later replay work can use structured parameters instead of treating rendered SQL as an exact replay artifact.

Bad:

- Modifying client-to-backend packet bytes.
- Emitting, storing, or broadcasting expanded SQL before redaction.
- Reconstructing raw binary bytes from binary summaries.
- Treating rendered SQL as dialect-perfect executable SQL.
- Logging raw SQL templates, raw parameter payloads, or decoded sensitive values.

### 6. Tests Required

For MySQL prepared statement expanded SQL rendering changes:

- Renderer test for quoted and escaped strings.
- Renderer test for `NULL`.
- Renderer tests for numeric, boolean, date/time/timestamp, JSON, binary summary, and unsupported values.
- Renderer test proving placeholders inside strings, identifiers, and comments are skipped.
- Renderer tests for missing and extra parameter errors.
- Adapter test proving known statement IDs store expanded SQL while observation remains byte-count only and emits zero events.
- Regression test coverage that existing numeric, string, binary, temporal, NULL bitmap, execute envelope, prepare, and query behavior remains unchanged.
- Run `cargo fmt --check`.
- Run `cargo test -p sql-lens-protocol-mysql`.
- Run `cargo test --workspace`.
- Run `cargo clippy --workspace --all-targets -- -D warnings`.

### 7. Wrong vs Correct

#### Wrong

```rust
packet.payload = render_expanded_sql(template, &parameters)?.into_bytes();
```

#### Correct

```rust
let expanded_sql = render_expanded_sql(template, &parameters)?;
envelope.expanded_sql = Some(expanded_sql);
```

## Scenario: SQL Event Redaction Before Storage And Broadcast

### 1. Scope / Trigger

- Trigger: SQL events are about to cross a retention or live delivery boundary.
- `sql-lens-core` owns the protocol-neutral redaction policy and event
  transformation.
- `sql-lens-storage` applies redaction before retaining events in the ring
  buffer.
- `sql-lens-api` applies redaction before publishing events to WebSocket
  subscribers.
- This layer must not parse SQL, add regex dependencies, classify PII, call
  plugins, mutate forwarded traffic, or change API response schemas.

### 2. Signatures

Core redaction APIs live in `crates/sql-lens-core/src/redaction.rs` and are
re-exported from the crate root:

```rust
pub const DEFAULT_REDACTION_MASK: &str = "***";
pub const DEFAULT_REDACTION_PARAMETER_NAMES: &[&str];

pub struct RedactionPolicy {
    pub enabled: bool,
    pub mask: String,
    pub parameter_names: Vec<String>,
    pub sql_patterns: Vec<String>,
}

pub fn redact_sql_event(event: SqlEvent, policy: &RedactionPolicy) -> SqlEvent;
```

Sink-boundary constructors must keep their default constructor and provide an
explicit policy constructor:

```rust
impl RingBufferStore {
    pub fn new(capacity: NonZeroUsize) -> Self;
    pub fn with_redaction_policy(
        capacity: NonZeroUsize,
        redaction_policy: RedactionPolicy,
    ) -> Self;
}

impl SqlEventBroadcaster {
    pub fn new(capacity: NonZeroUsize) -> Self;
    pub fn with_redaction_policy(
        capacity: NonZeroUsize,
        redaction_policy: RedactionPolicy,
    ) -> Self;
}
```

### 3. Contracts

- Redaction is enabled by default.
- The default mask is `***`.
- Default sensitive names are `password`, `passwd`, `token`, `secret`,
  `api_key`, `access_key`, and `refresh_token`.
- Parameter-name matching is case-insensitive exact matching.
- Empty configured parameter names are ignored.
- When a parameter name matches, set `redacted = true` and replace the value
  with `SqlParameterValue::String(mask)`.
- Parameters that arrive with `redacted = true` must stay redacted and must not
  retain their original value after sink-boundary redaction.
- Empty SQL patterns and empty sensitive values are ignored.
- SQL patterns are literal substring replacements across `original_sql`,
  `normalized_sql`, and `expanded_sql`.
- Redacted parameter values are also removed from `original_sql`,
  `normalized_sql`, and `expanded_sql` where simple display-value replacement
  can identify them.
- String-like parameter values should replace both raw text and single-quoted
  display literals using doubled single-quote escaping.
- `NULL` has no SQL text replacement candidate but the parameter value itself is
  still masked when the parameter is sensitive.
- `RingBufferStore::append` must call `redact_sql_event` before retaining the
  event.
- `SqlEventBroadcaster::publish` must call `redact_sql_event` before sending
  the event to subscribers.
- REST SQL event responses inherit storage redaction and must not duplicate
  redaction logic at serialization time in this layer.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| Policy disabled | Return the event unchanged |
| Parameter name matches with different case | Redact the parameter |
| Parameter already has `redacted = true` | Replace its value with the policy mask |
| SQL pattern is configured | Replace the literal pattern in all SQL text fields |
| SQL pattern is empty | Ignore it |
| Sensitive parameter value is empty | Do not replace whole SQL strings |
| Sensitive string value appears in expanded SQL | Replace it with the mask |
| Sensitive quoted display literal appears in expanded SQL | Replace it with quoted mask |
| Storage append receives sensitive event | Retain only the redacted event |
| WebSocket broadcaster receives sensitive event | Deliver only the redacted event |

### 5. Good/Base/Bad Cases

Good:

- Redaction rules are implemented once in `sql-lens-core`.
- Storage and WebSocket broadcast call the shared redactor at their sink
  boundaries.
- Tests assert the retained or delivered event no longer contains the raw
  sensitive value.

Base:

- Later app composition can convert `RedactionConfig` into `RedactionPolicy`.
- Later central capture fan-out can redact once before cloning events to
  storage, WebSocket, exporters, and statistics.
- Later security work can add SQL parsing, regex, classifiers, or plugin rules
  behind a new task design.

Bad:

- Adding `serde_json`, regex, SQL parser, async, database, or HTTP dependencies
  to `sql-lens-core` for this layer.
- Reimplementing separate redaction behavior in storage, API serializers, and
  WebSocket message builders.
- Logging raw SQL, parameters, authentication payloads, or database errors from
  redaction code.
- Changing `SqlEvent`, `SqlParameter`, or API response field names for masking.
- Treating redacted expanded SQL as executable replay SQL.

### 6. Tests Required

For SQL event redaction changes:

- Core tests for disabled policy, case-insensitive parameter-name matching,
  already-redacted parameters, SQL pattern replacement, quoted display literal
  replacement, and empty values.
- Config default tests proving the sensitive-name defaults match
  `SECURITY.md`.
- Storage tests proving `RingBufferStore::append` retains redacted parameters
  and redacted expanded SQL.
- Broadcaster or WebSocket tests proving live subscribers receive redacted
  events.
- Regression coverage that existing storage timeline/query behavior and
  WebSocket subscription behavior remain unchanged.
- Run `cargo fmt --check`.
- Run targeted crate tests for core, config, storage, and API.
- Run `cargo test --workspace`.
- Run `cargo clippy --workspace --all-targets -- -D warnings`.

### 7. Wrong vs Correct

#### Wrong

```rust
store.append(event);
broadcaster.publish(event.clone());
```

#### Correct

```rust
let event = redact_sql_event(event, &self.redaction_policy);
self.events.push_back(RingBufferEntry { sequence, event });
```

## Scenario: MySQL COM_QUERY Timing and Event Emission Contracts

### 1. Scope / Trigger

- Trigger: `sql-lens-protocol-mysql` measures a parsed `COM_QUERY` from client observation to a backend terminal query response.
- This layer is the first bridge from MySQL-specific command parsing to protocol-neutral `SqlEvent` emission.
- It must not parse detailed OK summaries, parse detailed ERR summaries, normalize SQL, fingerprint SQL, redact SQL, persist events, broadcast events, or block forwarding on capture consumers.

### 2. Signatures

MySQL-local timing types live in `crates/sql-lens-protocol-mysql/src/lib.rs`:

```rust
pub struct MysqlObservationTime {
    pub timestamp: sql_lens_core::Timestamp,
    pub monotonic: std::time::Instant,
}

pub trait MysqlObservationClock: std::fmt::Debug + Send + Sync {
    fn now(&self) -> MysqlObservationTime;
}

pub struct MysqlPendingQuery {
    pub command: MysqlClientCommand,
    pub started_at: sql_lens_core::Timestamp,
    pub started_monotonic: std::time::Instant,
}

impl MysqlProtocolAdapter {
    pub fn with_clock(clock: std::sync::Arc<dyn MysqlObservationClock>) -> Self;
}

impl MysqlConnectionState {
    pub fn pending_query(&self) -> Option<&MysqlPendingQuery>;
}
```

### 3. Contracts

- `MysqlProtocolAdapter::new()` uses a standard-library system clock only.
- Tests may inject a deterministic clock with `MysqlProtocolAdapter::with_clock`.
- `COM_QUERY` after `Authenticated` stores a pending query with SQL text, packet sequence ID, start timestamp, and start monotonic instant.
- Starting a new valid `COM_QUERY` replaces any existing pending query until result-set lifecycle support is added.
- `observe_client_bytes` emits zero events when starting pending timing.
- Backend payload first byte `0x00` finalizes the pending query with `CaptureStatus::Ok`.
- Backend payload first byte `0xff` finalizes the pending query with `CaptureStatus::Error`.
- Any unsupported backend payload keeps the pending query open and emits zero events.
- Backend terminal responses without a pending query emit zero events.
- Finalized query observation emits exactly one `SqlEvent` through `CaptureEventEmitter`.
- `ProtocolObservation.events_emitted` must equal the number of emitted events.
- Event IDs are deterministic process-local strings derived from connection ID and an incrementing per-connection query counter.
- Event connection fields come from `ProtocolConnectionContext.connection`.
- MySQL-only command details live under `ProtocolMetadata`, not top-level `SqlEvent` fields.
- Detailed OK result summaries and ERR packet summaries remain `None` until their dedicated tasks.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| Valid `COM_QUERY` before `Authenticated` | Count bytes only; do not start pending timing; emit zero events |
| Valid `COM_QUERY` after `Authenticated` | Store pending timing; keep phase `Authenticated`; emit zero events |
| Backend OK with pending query | Emit one `SqlEvent` with `CaptureStatus::Ok`; clear pending query |
| Backend ERR with pending query | Emit one `SqlEvent` with `CaptureStatus::Error`; clear pending query |
| Backend OK/ERR without pending query | Emit zero events |
| Unsupported backend response with pending query | Emit zero events; keep pending query |
| Malformed backend packet with pending query | Emit zero events; keep pending query |
| Monotonic elapsed duration exceeds `u64::MAX` milliseconds | Saturate event duration at `u64::MAX` |

### 5. Good/Base/Bad Cases

Good:

- A manual test clock provides `query_start` and `query_end`, and assertions verify exact duration and timing fields.
- `SqlEvent.metadata.fields` contains `command = "COM_QUERY"` and `command_sequence_id`.
- OK and ERR finalization both assert exactly one emitted event and a cleared pending query.

Base:

- Later OK packet parsing can populate `SqlEvent.result` without changing the pending timing contract.
- Later ERR packet parsing can populate `SqlEvent.error` without changing terminal status detection.
- Later result-set lifecycle parsing can replace first-byte OK handling for row-returning queries.

Bad:

- Adding MySQL command fields directly to `SqlEvent`.
- Introducing `time`, `chrono`, or `uuid` only for this timing layer.
- Parsing SQL text, rendering SQL, persisting events, or broadcasting WebSocket messages from the MySQL adapter.
- Treating unsupported backend packets as fatal during observation.

### 6. Tests Required

For MySQL `COM_QUERY` timing changes:

- Adapter test proving valid `COM_QUERY` after authentication starts pending timing and emits zero events.
- Adapter test proving backend OK finalizes pending timing, emits one OK event, records duration, and clears pending state.
- Adapter test proving backend ERR finalizes pending timing, emits one error event, records duration, and clears pending state.
- Adapter test proving terminal responses without pending timing emit zero events.
- Adapter test proving unsupported backend responses keep pending timing and emit zero events.
- Event assertions covering SQL text, connection context, duration, timing fields, and MySQL metadata.
- Run `cargo fmt --check`.
- Run `cargo test -p sql-lens-protocol-mysql`.
- Run `cargo test --workspace`.
- Run `cargo clippy --workspace --all-targets -- -D warnings`.

### 7. Wrong vs Correct

#### Wrong

```rust
event.metadata.fields.push(MetadataField {
    key: "mysql_ok_packet".to_owned(),
    value: MetadataValue::String(format!("{packet:?}")),
});
```

#### Correct

```rust
SqlEvent {
    status: CaptureStatus::Ok,
    result: None,
    metadata: ProtocolMetadata {
        protocol: ProtocolName("mysql".to_owned()),
        fields: vec![
            MetadataField {
                key: "command".to_owned(),
                value: MetadataValue::String("COM_QUERY".to_owned()),
            },
        ],
    },
    // remaining protocol-neutral fields copied from the connection context
}
```

## Scenario: MySQL OK Packet Summary Contracts

### 1. Scope / Trigger

- Trigger: `sql-lens-protocol-mysql` decodes basic command OK packet fields after `COM_QUERY` timing can emit successful events.
- This layer populates protocol-neutral `ResultSummary.affected_rows` and keeps MySQL status flags in metadata.
- It must not implement result-set lifecycle parsing, EOF-as-OK handling, warning/info/session-state decoding, storage, API, WebSocket, or UI behavior.

### 2. Signatures

Public OK parser contracts live in `crates/sql-lens-protocol-mysql/src/ok.rs` and are re-exported from the crate root:

```rust
pub struct MysqlOkPacketSummary {
    pub affected_rows: u64,
    pub status_flags: Option<u16>,
}

pub fn parse_ok_packet_summary(
    payload: &[u8],
) -> Result<Option<MysqlOkPacketSummary>, MysqlOkPacketParseError>;

pub enum MysqlOkPacketParseError {
    IncompletePayload { field: &'static str, needed: usize, available: usize },
    InvalidLengthEncodedInteger { field: &'static str, marker: u8 },
}
```

### 3. Contracts

- Command OK payloads with header `0x00` parse into `Some(MysqlOkPacketSummary)`.
- Non-OK payloads return `Ok(None)`.
- Empty payloads return `IncompletePayload { field: "header" }`.
- `affected_rows` is decoded as a MySQL length-encoded integer.
- `last_insert_id` is decoded only to advance the payload offset; it is not exposed in core models.
- `status_flags` is decoded as a 2-byte little-endian value when at least two bytes remain.
- One-byte, `0xfc`, `0xfd`, and `0xfe` length-encoded integer forms are supported.
- `0xfb` and `0xff` length-encoded integer markers are invalid for OK summary integer fields.
- Adapter-level OK summary parsing is non-fatal: malformed summaries still finalize the pending query as `CaptureStatus::Ok`, but leave `SqlEvent.result = None`.
- Successful OK summary parsing sets `SqlEvent.result = Some(ResultSummary { affected_rows: Some(value), returned_rows: None })`.
- Successful OK summary parsing adds `ok_status_flags` to MySQL metadata when status flags are present.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| Payload starts with `0x00`, affected rows `0`, status flags present | Return `Some` summary with `affected_rows = 0` and status flags |
| Payload starts with `0x00`, affected rows uses `0xfc` | Decode the 2-byte little-endian value |
| Payload starts with `0xff` | Return `Ok(None)` |
| Payload is empty | Return `IncompletePayload { field: "header" }` |
| Length-encoded integer marker is incomplete | Return `IncompletePayload` for the current field |
| Length-encoded integer marker is `0xfb` or `0xff` | Return `InvalidLengthEncodedInteger` |
| COM_QUERY backend OK summary parses | Emit OK event with affected rows and metadata status flags |
| COM_QUERY backend OK summary is malformed | Emit OK event with `result = None`; do not fail observation |
| COM_QUERY backend ERR packet is observed | Keep `result = None` |

### 5. Good/Base/Bad Cases

Good:

- Parser tests cover fixture OK payloads independently of adapter state.
- Adapter tests assert `ResultSummary.affected_rows`, `returned_rows = None`, and `ok_status_flags`.
- Malformed OK summaries are covered by an adapter regression test.

Base:

- Later OK parser tasks can expose warning count or info through MySQL metadata after a product requirement exists.
- Later result-set lifecycle work can populate `returned_rows` without changing affected-row OK parsing.

Bad:

- Adding `last_insert_id`, `status_flags`, or warning counts directly to `SqlEvent` or `ResultSummary`.
- Treating OK summary parse errors as protocol observation failures.
- Adding a broad MySQL binary codec abstraction before more packet families need it.
- Parsing packet payloads into logs or error messages.

### 6. Tests Required

For MySQL OK packet summary changes:

- Parser test for official-style OK packet with affected rows `0` and status flags.
- Parser test for non-zero affected rows.
- Parser tests for one-byte and `0xfc` length-encoded integer forms.
- Parser tests for incomplete and invalid length-encoded integer forms.
- Adapter test proving successful OK events include affected rows and status flag metadata.
- Adapter test proving malformed OK summaries stay non-fatal.
- Adapter test proving ERR events keep `result = None`.
- Run `cargo fmt --check`.
- Run `cargo test -p sql-lens-protocol-mysql`.
- Run `cargo test --workspace`.
- Run `cargo clippy --workspace --all-targets -- -D warnings`.

### 7. Wrong vs Correct

#### Wrong

```rust
pub struct SqlEvent {
    pub mysql_status_flags: u16,
    pub last_insert_id: u64,
}
```

#### Correct

```rust
SqlEvent {
    result: Some(ResultSummary {
        affected_rows: Some(summary.affected_rows),
        returned_rows: None,
    }),
    metadata: ProtocolMetadata {
        protocol: ProtocolName("mysql".to_owned()),
        fields: vec![MetadataField {
            key: "ok_status_flags".to_owned(),
            value: MetadataValue::Unsigned(u64::from(status_flags)),
        }],
    },
    // remaining event fields are protocol-neutral
}
```

## Scenario: MySQL ERR Packet Summary Contracts

### 1. Scope / Trigger

- Trigger: `sql-lens-protocol-mysql` decodes command ERR packet fields after `COM_QUERY` timing can emit failed events.
- This layer populates protocol-neutral `ErrorSummary` and keeps MySQL vendor error-code details in error metadata.
- It must not implement general SQL/PII redaction, error-code classification, authentication behavior changes, storage, API, WebSocket, or UI behavior.

### 2. Signatures

Public ERR parser contracts live in `crates/sql-lens-protocol-mysql/src/err.rs` and are re-exported from the crate root:

```rust
pub struct MysqlErrPacketSummary {
    pub error_code: u16,
    pub sql_state: Option<String>,
    pub message: String,
}

pub fn parse_err_packet_summary(
    payload: &[u8],
) -> Result<Option<MysqlErrPacketSummary>, MysqlErrPacketParseError>;

pub enum MysqlErrPacketParseError {
    IncompletePayload { field: &'static str, needed: usize, available: usize },
}
```

### 3. Contracts

- Command ERR payloads with header `0xff` parse into `Some(MysqlErrPacketSummary)`.
- Non-ERR payloads return `Ok(None)`.
- Empty payloads return `IncompletePayload { field: "header" }`.
- Error code is decoded as a 2-byte little-endian value.
- SQLSTATE is decoded only when the payload has `#` followed by five bytes.
- SQLSTATE bytes are decoded lossily to keep packet observation robust.
- Error message bytes are decoded with lossy UTF-8.
- Error message control characters are replaced before storing in `ErrorSummary`.
- Parser and adapter code must not log raw database error messages or packet payloads.
- Adapter-level ERR summary parsing is non-fatal: malformed summaries still finalize the pending query as `CaptureStatus::Error`, but leave `SqlEvent.error = None`.
- Successful ERR summary parsing sets `SqlEvent.error = Some(ErrorSummary { code, sql_state, message, metadata })`.
- `ErrorSummary.code` stores the MySQL vendor error code as a string.
- `ErrorSummary.metadata` includes `mysql_error_code` as a MySQL protocol metadata field.
- Failed query events keep `SqlEvent.result = None`.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| Payload starts with `0xff`, code, SQLSTATE, message | Return `Some` summary with all fields |
| Payload starts with `0xff`, code, message without `#` SQLSTATE marker | Return summary with `sql_state = None` |
| Payload starts with `0x00` | Return `Ok(None)` |
| Payload is empty | Return `IncompletePayload { field: "header" }` |
| Payload starts with `0xff` but error code is incomplete | Return `IncompletePayload { field: "error_code" }` |
| Payload starts with `0xff`, marker `#`, incomplete SQLSTATE | Return `IncompletePayload { field: "sql_state" }` |
| Message contains invalid UTF-8 bytes | Decode lossily and return a summary |
| Message contains control characters | Store sanitized message without control characters |
| COM_QUERY backend ERR summary parses | Emit error event with `ErrorSummary` |
| COM_QUERY backend ERR summary is malformed | Emit error event with `error = None`; do not fail observation |
| COM_QUERY backend OK packet is observed | Keep OK result summary behavior unchanged |

### 5. Good/Base/Bad Cases

Good:

- Parser tests cover official-style ERR payloads independently of adapter state.
- Adapter tests assert `ErrorSummary.code`, `sql_state`, sanitized `message`, and `mysql_error_code` metadata.
- Malformed ERR summaries are covered by an adapter regression test.

Base:

- Later redaction work can add stronger masking before persistence without changing packet field decoding.
- Later error-code classification can add derived metadata after a product requirement exists.

Bad:

- Adding `mysql_error_code` or `mysql_sql_state` directly to `SqlEvent`.
- Treating ERR summary parse errors as protocol observation failures.
- Logging raw database error messages from parser, adapter, tests, or runtime code.
- Adding broad redaction or error classification engines in the packet-summary task.

### 6. Tests Required

For MySQL ERR packet summary changes:

- Parser test for official-style ERR packet with error code, SQLSTATE, and message.
- Parser test for ERR packet without SQLSTATE.
- Parser tests for incomplete header, error code, and SQLSTATE.
- Parser test for lossy message decoding.
- Parser test for message control-character sanitization.
- Adapter test proving failed events include `ErrorSummary`.
- Adapter test proving malformed ERR summaries stay non-fatal.
- Adapter test proving OK events keep result summary behavior.
- Run `cargo fmt --check`.
- Run `cargo test -p sql-lens-protocol-mysql`.
- Run `cargo test --workspace`.
- Run `cargo clippy --workspace --all-targets -- -D warnings`.

### 7. Wrong vs Correct

#### Wrong

```rust
tracing::error!(message = summary.message, "mysql query failed");
```

#### Correct

```rust
SqlEvent {
    status: CaptureStatus::Error,
    result: None,
    error: Some(ErrorSummary {
        code: Some(summary.error_code.to_string()),
        sql_state: summary.sql_state,
        message: summary.message,
        metadata: Some(ProtocolMetadata {
            protocol: ProtocolName("mysql".to_owned()),
            fields: vec![MetadataField {
                key: "mysql_error_code".to_owned(),
                value: MetadataValue::Unsigned(u64::from(summary.error_code)),
            }],
        }),
    }),
    // remaining event fields are protocol-neutral
}
```

## Scenario: Proxy Graceful Shutdown Contracts

### 1. Scope / Trigger

- Trigger: `sql-lens-config` and `sql-lens-proxy` define the first shutdown coordination contract for the proxy runtime.
- Shutdown coordination stops accepts, notifies active sessions, and drains active session tasks within a bounded timeout.
- This layer must not install OS signal handlers, start the application runtime, parse protocols, emit capture events, persist lifecycle records, or allocate connection IDs.

### 2. Signatures

Config field:

```rust
pub struct ProxyConfig {
    pub shutdown_timeout_ms: u64,
}
```

Proxy shutdown types live in `crates/sql-lens-proxy/src/lib.rs`:

```rust
pub struct ProxyShutdownConfig {
    pub drain_timeout: std::time::Duration,
}

impl ProxyShutdownConfig {
    pub fn new(drain_timeout: std::time::Duration) -> Self;
    pub fn from_config(proxy: &sql_lens_config::ProxyConfig) -> Self;
}

pub struct ProxyShutdownSignal;

impl ProxyShutdownSignal {
    pub fn new() -> Self;
    pub fn subscribe(&self) -> tokio::sync::watch::Receiver<bool>;
    pub fn request_shutdown(&self) -> Result<(), ProxyShutdownError>;
}

pub struct ActiveSessionDrain;

impl ActiveSessionDrain {
    pub async fn drain<T>(
        sessions: Vec<tokio::task::JoinHandle<T>>,
        config: &ProxyShutdownConfig,
    ) -> ShutdownDrainSummary
    where
        T: Send + 'static;
}
```

Allowed dependencies remain:

```toml
sql-lens-config = { path = "../sql-lens-config" }
tokio = { version = "1", features = ["net", "sync", "time", "rt", "macros", "io-util"] }
tracing = "0.1"
```

Do not add `tokio-util`, signal handling crates, app crates, storage crates, protocol crates, or lifecycle ID dependencies for this layer.

### 3. Contracts

- `ProxyConfig.shutdown_timeout_ms` defaults to `10_000`.
- `ProxyShutdownConfig::from_config` maps `shutdown_timeout_ms` to `Duration::from_millis`.
- `ProxyShutdownSignal` uses `watch<bool>` where `false` means running and `true` means shutdown requested.
- Listener shutdown should continue to use `watch::Receiver<bool>`; no second listener shutdown mechanism.
- `ActiveSessionDrain::drain` waits for active session `JoinHandle`s until the configured drain timeout.
- A joined task with `Ok(_)` counts as completed.
- A joined task with `Err(_)` counts as failed.
- On timeout, remaining tasks are aborted and counted as timed out.
- Drain timeout is represented in `ShutdownDrainSummary`, not as an exception.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| Missing `proxy.shutdown_timeout_ms` in TOML | Use default `10_000` |
| TOML sets `proxy.shutdown_timeout_ms` | Parse the configured value |
| Shutdown signal has active receivers | `request_shutdown` sends `true` |
| Shutdown signal has no receivers | Return `ProxyShutdownError::NoReceivers` |
| Session task completes before timeout | Increment `completed_sessions` |
| Session task joins with error before timeout | Increment `failed_sessions` |
| Drain timeout elapses | Abort remaining tasks, set `timed_out = true`, increment `timed_out_sessions` |
| Later OS signal support is needed | Implement in `sql-lens-app`, not in proxy primitives |

### 5. Good/Base/Bad Cases

Good:

- One `ProxyShutdownSignal` fans out receivers to listener and active sessions.
- Tests use short drain timeouts and pending tasks to prove abort-on-timeout.
- Runtime composition later calls listener shutdown and `ActiveSessionDrain` without changing the primitive contracts.

Base:

- A clean local shutdown drains all completed forwarding sessions and reports counts.
- A stuck session is aborted only after the configured drain timeout.

Bad:

- Calling `tokio::signal::ctrl_c` inside `sql-lens-proxy`.
- Blocking shutdown drain on storage, capture, plugin hooks, or WebSocket clients.
- Treating shutdown timeout as idle timeout.
- Creating connection lifecycle records from the shutdown primitive.

### 6. Tests Required

For proxy graceful shutdown changes:

- Config default test for `shutdown_timeout_ms`.
- TOML override test for `proxy.shutdown_timeout_ms`.
- `ProxyShutdownConfig::from_config` test.
- Shutdown signal notification test.
- Listener stop test using `ProxyShutdownSignal`.
- Successful active-session drain test.
- Failed active-session drain test.
- Drain timeout and abort test.
- Run socket-binding tests outside sandboxes that deny local TCP binds.
- Run `cargo fmt --check`.
- Run `cargo check --workspace`.
- Run `cargo test --workspace`.
- Run `cargo clippy --workspace --all-targets -- -D warnings`.

### 7. Wrong vs Correct

#### Wrong

```rust
tokio::signal::ctrl_c().await?;
storage.write_connection_closed(id).await?;
```

#### Correct

```rust
let shutdown = ProxyShutdownSignal::new();
let listener_shutdown = shutdown.subscribe();
shutdown.request_shutdown()?;
let summary = ActiveSessionDrain::drain(session_handles, &shutdown_config).await;
```

## Scenario: HTTP API Server Foundation Contracts

### 1. Scope / Trigger

- Trigger: `sql-lens-api` owns the first HTTP server primitive for the Web/API surface.
- The foundation must let later REST, WebSocket, static web, and dashboard work compose routes without changing listener and request-correlation contracts.
- This layer must not start the application runtime, install OS signal handlers, parse SQL protocols, query storage, or define product endpoints that are owned by later API tasks.

### 2. Signatures

HTTP server types live in `crates/sql-lens-api/src/` and are re-exported from `lib.rs`:

```rust
pub const REQUEST_ID_HEADER: &str = "x-request-id";

pub struct HttpServerConfig {
    pub listen: String,
    pub request_timeout_ms: u64,
}

impl From<&sql_lens_config::WebConfig> for HttpServerConfig;

pub struct BoundHttpServer;

impl BoundHttpServer {
    pub fn local_addr(&self) -> std::net::SocketAddr;

    pub async fn serve_with_shutdown(
        self,
        shutdown: impl std::future::Future<Output = ()> + Send + 'static,
    ) -> Result<(), HttpServerError>;
}

pub async fn bind_http_server(
    config: &HttpServerConfig,
) -> Result<BoundHttpServer, HttpServerError>;

pub fn router() -> axum::Router;
```

Allowed dependencies for this first API server layer:

```toml
axum = "0.8"
sql-lens-config = { path = "../sql-lens-config" }
tokio = { version = "1", features = ["net"] }

[dev-dependencies]
tokio = { version = "1", features = ["macros", "rt", "sync", "time"] }
tower = { version = "0.5", features = ["util"] }
```

Do not add `uuid`, `time`, storage crates, proxy crates, protocol crates, TLS dependencies, or OpenAPI generation dependencies to this foundation layer without a task-level design update.

### 3. Contracts

- `HttpServerConfig::from(&WebConfig)` copies `web.listen` and `web.request_timeout_ms`.
- `bind_http_server` binds a `tokio::net::TcpListener` using the configured listen address.
- Tests and later runtime composition discover the actual address with `BoundHttpServer::local_addr`.
- `serve_with_shutdown` uses Axum graceful shutdown and a caller-owned shutdown future.
- `sql-lens-api` does not own OS signal handling; later application composition belongs in `sql-lens-app`.
- `router()` installs request ID middleware but does not define product endpoints by default.
- The request ID header is `x-request-id`.
- If a request includes `x-request-id`, the same value is exposed to handlers and propagated to the response.
- If a request omits `x-request-id`, middleware generates a dependency-light process-local ID suitable for correlation, not security.
- Request IDs may be stored in request extensions for handlers.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| `web.listen` is valid and bindable | Return `BoundHttpServer` |
| `web.listen` cannot be bound | Return `HttpServerError::Bind` with the configured listen value and IO source |
| bound local address cannot be read | Return `HttpServerError::LocalAddr` |
| Axum server returns an IO error | Return `HttpServerError::Serve` |
| shutdown future resolves | Stop accepting new connections and drain through Axum graceful shutdown |
| request includes `x-request-id` | Preserve and propagate that value |
| request omits `x-request-id` | Generate and propagate an ID |
| a product endpoint is needed | Add it in a dedicated endpoint task, not as part of foundation cleanup |

### 5. Good/Base/Bad Cases

Good:

- API tests bind to `127.0.0.1:0` and assert `local_addr().port() != 0`.
- Middleware tests call the router through `tower::ServiceExt::oneshot`.
- Runtime composition later owns Ctrl-C handling and passes a shutdown future into `serve_with_shutdown`.

Base:

- An empty foundation router returns 404 while still attaching `x-request-id`.
- A caller converts `&config.web` into `HttpServerConfig` and then binds the server.

Bad:

- Adding `/api/v1/health` in a server foundation task when a dedicated health endpoint task exists.
- Starting the HTTP server directly from `sql-lens-api`; application runtime
  composition belongs in `sql-lens-app`.
- Using cryptographic request ID dependencies before a security task requires them.
- Putting proxy, storage, protocol parser, or SQL replay logic inside `sql-lens-api`.

### 6. Tests Required

For HTTP server foundation changes:

- `HttpServerConfig::from(&WebConfig)` field mapping test.
- Bind test using an ephemeral port.
- Graceful shutdown test using a caller-provided future.
- Generated request ID response header test.
- Incoming request ID propagation test.
- Runtime composition changes should be tested through `sql-lens-app`, while
  `sql-lens-api` tests stay focused on listener and router primitives.
- Run `cargo fmt --check`.
- Run `cargo check --workspace`.
- Run `cargo test --workspace`.
- Run `cargo clippy --workspace --all-targets -- -D warnings`.

### 7. Wrong vs Correct

#### Wrong

```rust
#[tokio::main]
async fn main() {
    let app = sql_lens_api::router()
        .route("/api/v1/health", get(health));
    axum::serve(listener, app).await.unwrap();
}
```

#### Correct

```rust
let server_config = HttpServerConfig::from(&config.web);
let server = bind_http_server(&server_config).await?;
let local_addr = server.local_addr();
server.serve_with_shutdown(shutdown_future).await?;
```

## Scenario: WebSocket Server Foundation Contracts

### 1. Scope / Trigger

- Trigger: `sql-lens-api` owns the first WebSocket upgrade endpoint for live SQL event streaming.
- The foundation must register `GET /ws/sql`, accept upgrades, send a minimal heartbeat, and handle disconnects cleanly.
- This layer must not implement SQL event fan-out, subscription parsing, filters, replay, statistics streaming, or frontend code.

### 2. Signatures

WebSocket route code lives in `crates/sql-lens-api/src/websocket.rs` and is merged by `server::router_with_state` before fallback:

```rust
pub const SQL_WS_PATH: &str = "/ws/sql";
```

Allowed dependency changes for this foundation:

```toml
axum = { version = "0.8", features = ["ws"] }

[dev-dependencies]
futures-util = "0.3"
tokio-tungstenite = "0.28"
```

Do not add capture, storage broadcast, OpenAPI, TLS, or frontend dependencies for the WebSocket foundation task.

### 3. Contracts

- `GET /ws/sql` uses Axum `WebSocketUpgrade`.
- A valid WebSocket upgrade returns a switching-protocols response.
- Request ID middleware still applies to the HTTP upgrade response.
- After upgrade, the server sends one initial `Message::Ping` heartbeat.
- The socket reads until the client sends close, disconnects, or a socket error occurs.
- Text, binary, ping, and pong frames are ignored until a later subscription protocol task defines message schemas.
- Plain HTTP requests to `/ws/sql` may use Axum's WebSocket extractor rejection instead of the REST JSON error envelope.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| Valid WebSocket upgrade | Return switching protocols and spawn socket lifecycle |
| Socket opens successfully | Send one initial ping heartbeat |
| Initial ping send fails | Treat as clean early disconnect |
| Client sends close | End socket lifecycle without panic |
| Socket read returns error | End socket lifecycle without API error mapping |
| Plain HTTP request hits `/ws/sql` | Return non-200 Axum upgrade rejection |

### 5. Good/Base/Bad Cases

Good:

- Use a real local server and WebSocket client in tests for upgrade behavior.
- Keep heartbeat to one initial ping until timeout policy is explicitly designed.
- Keep subscription JSON parsing in a later task.

Base:

- `router_with_state` merges WebSocket routes before fallback and under request ID middleware.
- The endpoint is protocol-neutral and does not mention MySQL-specific event fields.

Bad:

- Adding event broadcast channels, storage fan-out, or replay behavior to the foundation endpoint.
- Blocking WebSocket socket reads on capture or storage writes.
- Forcing WebSocket upgrade failures through REST error envelopes without a protocol-level error design.

### 6. Tests Required

For WebSocket foundation changes:

- Valid WebSocket upgrade test using `GET /ws/sql`.
- Initial ping heartbeat test.
- Clean close/disconnect test.
- Plain HTTP `/ws/sql` rejection test.
- Existing REST endpoint tests still pass.
- Run `cargo fmt --check`.
- Run `cargo test -p sql-lens-api`.
- Run `cargo test --workspace`.
- Run `cargo clippy --workspace --all-targets -- -D warnings`.

## Scenario: SQL WebSocket Subscription Contracts

### 1. Scope / Trigger

- Trigger: `sql-lens-api` broadcasts live `SqlEvent` values to `/ws/sql` subscribers.
- This layer owns API-local WebSocket fan-out, subscription filtering, and message serialization.
- It must not wire proxy/runtime capture receivers, replay history, or add MySQL-specific behavior.

### 2. Signatures

API-local broadcast types live in `crates/sql-lens-api/src/live_sql_events.rs` and are re-exported from `lib.rs`:

```rust
pub const DEFAULT_SQL_EVENT_BROADCAST_CAPACITY: usize = 1024;

pub struct SqlEventBroadcaster;
pub struct SqlEventSubscription;

impl SqlEventBroadcaster {
    pub fn new(capacity: std::num::NonZeroUsize) -> Self;
    pub fn publish(&self, event: sql_lens_core::SqlEvent) -> SqlEventBroadcastOutcome;
    pub fn subscribe(&self) -> SqlEventSubscription;
    pub fn subscriber_count(&self) -> usize;
    pub fn stats(&self) -> SqlEventBroadcastStats;
}
```

Allowed dependency shape:

```toml
serde_json = "1.0"
tokio = { version = "1", features = ["net", "sync"] }
```

Do not add capture, proxy, app runtime, filter engines, OpenAPI, or frontend dependencies for this subscription layer.

### 3. Contracts

- `ApiState` owns a `SqlEventBroadcaster` and exposes a cloneable accessor for tests and future runtime composition.
- `SqlEventBroadcaster` uses `tokio::sync::broadcast` so multiple WebSocket clients can receive the same live event without blocking publishers.
- `publish` is non-async and must not wait on WebSocket clients.
- If there are no subscribers, `publish` returns `NoSubscribers` and records the condition without treating it as a hard error.
- A socket must send a valid JSON text message with `type = "subscribe"` and `version = 1` before receiving SQL events.
- Malformed, unsupported, or wrong-version subscription messages are ignored while the socket continues waiting for a valid subscribe.
- Subscribe messages may include an optional `filters` object.
- Supported WebSocket filters are `protocol`, `status`, `database`, `min_duration_ms`, and `max_duration_ms`.
- Unknown filter fields are invalid.
- `protocol` and `database` use exact string matches against `SqlEvent`.
- `status` must be a non-empty array containing only `ok`, `slow`, `error`, and `unknown`.
- Multiple status values use OR semantics; different filter fields use AND semantics.
- Duration bounds are inclusive and must satisfy `min_duration_ms <= max_duration_ms` when both are present.
- Invalid filters send a `subscription.error` JSON text frame with `version: 1`, `payload.code: "INVALID_FILTER"`, and `payload.field` pointing at the invalid filter.
- After a subscription filter error, the socket remains open and continues waiting for a later valid subscribe message.
- After subscription, server messages are JSON text frames with `type`, `version`, and `payload`.
- `sql_event.created` payloads reuse `SqlEventSummaryResponse` mapping.
- Subscriber lag is local to that subscriber; lagged receivers skip missed events and continue with newer retained events.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| Publish with one active subscriber | Return `Delivered { subscriber_count: 1 }`; subscriber can receive the event |
| Publish with no subscribers | Return `NoSubscribers`; do not fail the caller |
| Subscriber lags beyond broadcast capacity | Report lag locally and continue reading retained events |
| Socket has not sent valid subscribe | Do not forward SQL events to that socket |
| Socket sends malformed subscribe text | Ignore and keep waiting |
| Socket sends valid subscribe without filters | Start forwarding all future broadcast SQL events |
| Socket sends valid subscribe with filters | Start forwarding only matching future broadcast SQL events |
| Socket sends subscribe with invalid filters | Send `subscription.error`; keep waiting for valid subscribe |
| Socket receives matching live event | Send `sql_event.created` JSON text with `version: 1` |
| Socket receives non-matching live event | Skip it silently for that subscriber |

### 5. Good/Base/Bad Cases

Good:

- Tests publish through `ApiState::sql_event_broadcaster()` into a running `/ws/sql` server.
- WebSocket payloads reuse the REST SQL event summary DTO.
- Invalid top-level subscribe messages do not close the socket.
- Invalid filter messages use the subscription error envelope and do not close the socket.

Base:

- Future `sql-lens-app` runtime fan-out can read from `CaptureEventReceiver` and call `SqlEventBroadcaster::publish`.
- Future filter fields should stay protocol-neutral or live under protocol metadata instead of adding MySQL-specific top-level behavior.

Bad:

- Letting each WebSocket client read directly from the single-consumer capture `mpsc` receiver.
- Sending historical ring-buffer events as part of live subscription.
- Blocking publishers until slow WebSocket clients read messages.
- Adding MySQL-specific fields to WebSocket message envelopes.
- Treating invalid filter values as unfiltered subscriptions.

### 6. Tests Required

For SQL WebSocket subscription changes:

- Broadcast unit test for delivery to one subscriber.
- Broadcast unit test for no-subscriber publish behavior.
- Broadcast lag test.
- WebSocket test proving events are not sent before valid subscribe.
- WebSocket test proving invalid subscribe messages are ignored and valid subscribe can still succeed.
- WebSocket test proving a subscribed client receives `sql_event.created`.
- WebSocket tests proving protocol, status, database, and duration filters send matching events only.
- WebSocket test proving invalid filters return `subscription.error`.
- WebSocket test proving valid subscribe can still succeed after a filter error.
- Run `cargo fmt --check`.
- Run `cargo test -p sql-lens-api`.
- Run `cargo test --workspace`.
- Run `cargo clippy --workspace --all-targets -- -D warnings`.

## Scenario: REST Error Response Contracts

### 1. Scope / Trigger

- Trigger: `sql-lens-api` handlers return standardized REST API errors through `ApiEndpointError`.
- REST error responses must match the `API.md` `ApiError` envelope.
- Request ID behavior is shared by all API endpoints through the request ID middleware.
- This layer does not implement rate limiting, storage health, proxy readiness, panic recovery, or OpenAPI generation by itself.

### 2. Signatures

Public JSON shape:

```json
{
  "error": {
    "code": "BAD_REQUEST",
    "message": "Invalid duration filter",
    "request_id": "sql-lens-0000000000000001",
    "details": {
      "field": "min_duration_ms"
    }
  }
}
```

Internal error type:

```rust
pub(crate) struct ApiEndpointError;

impl ApiEndpointError {
    pub(crate) fn bad_request(message: impl Into<String>, field: impl Into<String>) -> Self;
    pub(crate) fn not_found(message: impl Into<String>, key: impl Into<String>, value: impl Into<String>) -> Self;
}
```

Do not add unused future-facing constructors just to mirror `ApiErrorCode`; keep complete status/code-name mappings tested, and add constructors when runtime code first needs them.

Request ID middleware contract:

```rust
pub struct RequestId;

impl RequestId {
    pub fn as_header_value(&self) -> &axum::http::HeaderValue;
    pub(crate) fn as_str(&self) -> &str;
}
```

### 3. Contracts

- `BAD_REQUEST` maps to HTTP 400.
- `NOT_FOUND` maps to HTTP 404.
- `CONFLICT` maps to HTTP 409.
- `RATE_LIMITED` maps to HTTP 429.
- `INTERNAL` maps to HTTP 500.
- `STORAGE_UNAVAILABLE` maps to HTTP 503.
- `PROXY_NOT_READY` maps to HTTP 503.
- Error responses include `x-request-id` response header.
- Error response JSON includes the same request ID string in `error.request_id`.
- Valid incoming `x-request-id` values are preserved in both response header and error body.
- Invalid incoming `x-request-id` values are replaced with a generated `sql-lens-*` request ID.
- `router_with_state` uses a fallback that returns a `NOT_FOUND` API envelope for unmatched routes.
- `ApiEndpointError` marks its own responses through a typed response extension so request ID middleware can inject the body request ID without parsing JSON.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| Handler returns `ApiEndpointError::bad_request` | HTTP 400, code `BAD_REQUEST`, request ID header and body present |
| Each documented `ApiErrorCode` mapping is tested | HTTP status and string code match `API.md` |
| Client sends valid `x-request-id` | Preserve that value in header and error body |
| Client sends invalid `x-request-id` | Generate a replacement request ID used in header and error body |
| Request path matches no route | Return HTTP 404 with code `NOT_FOUND` and `details.path` |

### 5. Good/Base/Bad Cases

Good:

- Endpoint handlers continue returning `Result<Json<T>, ApiEndpointError>`.
- Request ID injection stays centralized in request ID middleware.
- Tests parse representative error JSON bodies instead of checking only headers.

Base:

- Unknown routes use the same error envelope as handler errors.
- Constructors exist before rate-limit/storage/proxy runtime code needs them.

Bad:

- Setting `request_id: None` in API error bodies routed through the app.
- Duplicating request ID plumbing in every handler signature.
- Parsing JSON response bodies in middleware when a typed response extension can carry the same information.
- Adding a new public error code such as `METHOD_NOT_ALLOWED` without a product/API contract update.

### 6. Tests Required

For REST error response changes:

- Mapping test for every documented `ApiErrorCode`.
- Generated request ID appears in both header and error body.
- Incoming valid request ID appears in both header and error body.
- Invalid incoming request ID is replaced.
- Unmatched route returns `NOT_FOUND` envelope.
- Representative handler error, such as invalid query parameters, includes body request ID.
- Existing success endpoint tests still pass.
- Run `cargo fmt --check`.
- Run `cargo check --workspace`.
- Run `cargo test --workspace`.
- Run `cargo clippy --workspace --all-targets -- -D warnings`.

## Scenario: OpenAPI Generation Contracts

### 1. Scope / Trigger

- Trigger: `sql-lens-api` changes REST DTOs, paths, query parameters, request bodies, response bodies, or shared API error envelopes.
- The generated document at `docs/openapi/sql-lens.v1.yaml` is a public frontend and release contract.
- WebSocket routes are not REST endpoints and should not be added as OpenAPI paths.

### 2. Signatures

Generation command:

```bash
rtk cargo run -p sql-lens-api --example generate-openapi > docs/openapi/sql-lens.v1.yaml
```

Public API helpers:

```rust
pub fn openapi() -> utoipa::openapi::OpenApi;
pub fn openapi_yaml() -> Result<String, Box<dyn std::error::Error + Send + Sync + 'static>>;
```

### 3. Contracts

- REST response/request DTOs derive `utoipa::ToSchema` when they are part of the public API contract.
- The OpenAPI aggregate lives in `sql-lens-api`; app/runtime crates do not generate the document.
- OpenAPI marker functions may describe routes without being runtime handlers.
- Shared `ApiErrorEnvelope` is included as a schema for non-2xx REST responses.
- The committed YAML must be regenerated in the same change that modifies public REST API shape.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| A REST endpoint is added or removed | Update the OpenAPI path list and regenerated YAML |
| A REST DTO field changes | Update schema derives or hints and regenerated YAML |
| Protocol metadata appears in REST DTOs | Keep it schema-compatible as a JSON object unless a stable typed schema exists |
| Generated YAML differs from the committed file | Fail the stale-output test with the regeneration command |

### 5. Tests Required

For OpenAPI or REST contract changes:

- Test that all implemented REST paths are present in the generated document.
- Test that generated YAML exactly matches `docs/openapi/sql-lens.v1.yaml`.
- Run `cargo fmt --check`.
- Run `cargo test -p sql-lens-api`.
- Run `cargo test --workspace`.
- Run `cargo clippy --workspace --all-targets -- -D warnings`.

## Scenario: Health Endpoint Contracts

### 1. Scope / Trigger

- Trigger: `sql-lens-api` defines the first REST endpoint contract: `GET /api/v1/health`.
- The endpoint is a lightweight local readiness signal for development tools, smoke tests, and future UI/runtime composition.
- It must not become a deep dependency readiness check until storage/proxy runtime composition exists.

### 2. Signatures

Health endpoint contract:

```http
GET /api/v1/health
```

Public API types:

```rust
pub const HEALTH_PATH: &str = "/api/v1/health";

pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_ms: u64,
}

pub struct HealthState;
```

Response body:

```json
{
  "status": "ok",
  "version": "0.1.0",
  "uptime_ms": 120000
}
```

### 3. Contracts

- `router()` registers `GET /api/v1/health`.
- Successful health responses return HTTP 200.
- `status` is `"ok"` for the first implementation.
- `version` comes from the API crate package version through `env!("CARGO_PKG_VERSION")`.
- `uptime_ms` is computed from `std::time::Instant`, not wall-clock time.
- `uptime_ms` is saturated to `u64::MAX` if elapsed milliseconds ever exceed `u64`.
- The endpoint works without storage, proxy, capture, protocol adapter, plugin, frontend, or database state.
- Request ID middleware applies to health responses.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| `GET /api/v1/health` | Return HTTP 200 and JSON `HealthResponse` |
| storage is unavailable or uninitialized | Health endpoint still returns basic process health |
| request omits `x-request-id` | Response still includes generated `x-request-id` |
| uptime cannot fit in `u64` | Return `u64::MAX` |
| deep component readiness is needed | Add a separate readiness task and response contract |

### 5. Good/Base/Bad Cases

Good:

- Health schema tests deserialize the response into `HealthResponse`.
- The endpoint can be called through `tower::ServiceExt::oneshot` without starting a TCP listener.

Base:

- A newly created router immediately reports `"ok"` with a small `uptime_ms`.

Bad:

- Querying storage or proxy state from the health handler before runtime composition exists.
- Returning ad hoc JSON fields that do not match `API.md`.
- Changing `sql-lens-app` into a long-running server just to test the handler.

### 6. Tests Required

For health endpoint changes:

- HTTP 200 test.
- Response deserializes into `HealthResponse`.
- `status == "ok"`.
- `version == env!("CARGO_PKG_VERSION")`.
- `uptime_ms` is numeric.
- Response includes `x-request-id`.
- Existing request ID and server foundation tests still pass.
- Run `cargo fmt --check`.
- Run `cargo check --workspace`.
- Run `cargo test --workspace`.
- Run `cargo clippy --workspace --all-targets -- -D warnings`.

### 7. Wrong vs Correct

#### Wrong

```rust
async fn health(storage: Storage) -> Json<serde_json::Value> {
    Json(json!({ "database": storage.ping().await }))
}
```

#### Correct

```rust
async fn health(Extension(state): Extension<HealthState>) -> Json<HealthResponse> {
    Json(state.snapshot())
}
```

## Scenario: SQL Event List Endpoint Contracts

### 1. Scope / Trigger

- Trigger: `sql-lens-api` exposes retained SQL events through `GET /api/v1/sql-events`.
- The endpoint is the API-facing SQL timeline list and must map HTTP query parameters into strongly typed storage filters.
- This layer formats response DTOs for `API.md`; it must not leak Rust enum variant names, internal ring-buffer sequence numbers, or storage structs directly.

### 2. Signatures

Endpoint:

```http
GET /api/v1/sql-events
```

Router and state signatures:

```rust
pub struct ApiState;

impl ApiState {
    pub fn new(event_store: sql_lens_storage::RingBufferStore) -> Self;
    pub fn event_store(&self) -> std::sync::Arc<tokio::sync::RwLock<sql_lens_storage::RingBufferStore>>;
    pub fn with_sqlite_event_reader(
        event_store: sql_lens_storage::RingBufferStore,
        sqlite_store: sql_lens_storage::SqliteEventStore,
    ) -> Self;
}

pub fn router() -> axum::Router;
pub fn router_with_state(state: ApiState) -> axum::Router;
```

Public response DTOs:

```rust
pub const SQL_EVENTS_PATH: &str = "/api/v1/sql-events";

pub struct SqlEventListResponse {
    pub items: Vec<SqlEventSummaryResponse>,
    pub next_cursor: Option<String>,
}

pub struct SqlEventSummaryResponse {
    pub id: String,
    pub timestamp: String,
    pub protocol: String,
    pub database_type: String,
    pub connection_id: String,
    pub client_addr: String,
    pub backend_addr: String,
    pub user: Option<String>,
    pub database: Option<String>,
    pub kind: String,
    pub status: String,
    pub duration_ms: u64,
    pub original_sql: String,
    pub expanded_sql: Option<String>,
    pub fingerprint: Option<String>,
    pub rows: Option<RowsSummaryResponse>,
    pub metadata: ProtocolMetadataResponse,
}
```

Allowed `sql-lens-api` dependencies for this layer:

```toml
sql-lens-core = { path = "../sql-lens-core" }
sql-lens-storage = { path = "../sql-lens-storage" }
tokio = { version = "1", features = ["net", "sync"] }
```

### 3. Contracts

- `router()` uses `ApiState::default()` and remains usable for empty API smoke tests.
- `router_with_state(ApiState)` registers health routes and SQL event routes under the existing request ID middleware.
- `ApiState` stores the live `RingBufferStore` behind `Arc<RwLock<_>>` and uses
  a configured SQL event read source for REST timeline/detail/export/replay
  event lookups.
- The endpoint supports `limit`, `cursor`, `protocol`, `database_type`, `database`, `user`, `client_addr`, `status`, `min_duration_ms`, `max_duration_ms`, `q`, `fingerprint`, `from`, and `to`.
- Default `limit` is `100`.
- Maximum `limit` is `500`; larger values are clamped to `500`.
- `limit = 0` returns HTTP 400.
- Cursor format is `seq_<u64>`.
- Invalid cursor returns HTTP 400.
- Status values are `ok`, `slow`, `error`, and `unknown`.
- Response event `kind` values are snake_case strings such as `statement_execute`.
- Response event `status` values are lowercase strings.
- Metadata is returned as a deterministic protocol-keyed object, not as `Vec<MetadataField>`.
- API errors use the documented error envelope and core `ApiErrorCode` names.
- Request ID headers are still attached to successful and error responses.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| Store is empty | Return HTTP 200 with empty `items` and `next_cursor: null` |
| Retained events exist | Return newest matching events first |
| `limit` is present | Return at most that many items, clamped to `500` |
| `limit = 0` | Return HTTP 400 `BAD_REQUEST` with `field = "limit"` |
| Older matching events exist after a page | Return `next_cursor` |
| `cursor = seq_N` | Return retained matching events with internal sequence `< N` |
| Cursor has an unsupported format | Return HTTP 400 `BAD_REQUEST` with `field = "cursor"` |
| `min_duration_ms > max_duration_ms` | Return HTTP 400 `BAD_REQUEST` with `field = "min_duration_ms"` |
| `from > to` | Return HTTP 400 `BAD_REQUEST` with `field = "from"` |
| Request omits `x-request-id` | Response includes a generated `x-request-id` header |

### 5. Good/Base/Bad Cases

Good:

- Endpoint tests inject a `RingBufferStore` through `router_with_state`.
- DTO tests assert lowercase/snake_case `status` and `kind` values.
- Metadata tests assert protocol-keyed JSON such as `metadata.mysql.command`.

Base:

- Ring-buffer-only runtime creates one configured ring buffer and passes it to
  `ApiState::new`.
- SQLite runtime creates one configured ring buffer for live state and passes a
  configured `SqliteEventStore` to `ApiState::with_sqlite_event_reader`.

Bad:

- Returning `SqlEvent` directly and exposing Rust enum names such as `StatementExecute`.
- Exposing raw ring-buffer sequence numbers as JSON numbers instead of encoded cursors.
- Adding SQL parsing, database connections, or runtime startup to the endpoint task.
- Holding the storage write lock while serializing response bodies.

### 6. Tests Required

For SQL event list endpoint changes:

- Empty list response test.
- Populated list schema test matching `API.md` fields.
- Query-parameter-to-storage-filter test.
- Cursor pagination test.
- Invalid cursor HTTP 400 test.
- Invalid duration range HTTP 400 test.
- Storage tests for `client_addr` and `fingerprint` filters.
- Existing health and request ID tests still pass.
- Run `cargo fmt --check`.
- Run `cargo check --workspace`.
- Run `cargo test --workspace`.
- Run `cargo clippy --workspace --all-targets -- -D warnings`.

### 7. Wrong vs Correct

#### Wrong

```rust
async fn list() -> Json<Vec<SqlEvent>> {
    Json(store.snapshot())
}
```

#### Correct

```rust
async fn list_sql_events(
    Extension(state): Extension<ApiState>,
    Query(params): Query<SqlEventListQueryParams>,
) -> Result<Json<SqlEventListResponse>, ApiEndpointError> {
    let query = params.try_into_timeline_query()?;
    let page = {
        let event_store = state.event_store();
        let store = event_store.read().await;
        store.query_timeline(query)?
    };

    Ok(Json(SqlEventListResponse {
        items: page.events.iter().map(SqlEventSummaryResponse::from).collect(),
        next_cursor: page.next_cursor.map(encode_cursor),
    }))
}
```

## Scenario: SQL Event Detail Endpoint Contracts

### 1. Scope / Trigger

- Trigger: `sql-lens-api` exposes retained SQL event detail through `GET /api/v1/sql-events/{id}`.
- The endpoint backs the future SQL detail panel and must include the complete retained event payload needed for inspection.
- It must reuse the SQL event API mapping conventions instead of creating a second response style.

### 2. Signatures

Endpoint:

```http
GET /api/v1/sql-events/{id}
```

Public constants and DTOs:

```rust
pub const SQL_EVENT_DETAIL_PATH: &str = "/api/v1/sql-events/{id}";

pub struct SqlEventDetailResponse {
    pub id: String,
    pub timestamp: String,
    pub protocol: String,
    pub database_type: String,
    pub connection_id: String,
    pub client_addr: String,
    pub backend_addr: String,
    pub user: Option<String>,
    pub database: Option<String>,
    pub kind: String,
    pub status: String,
    pub duration_ms: u64,
    pub original_sql: String,
    pub normalized_sql: Option<String>,
    pub expanded_sql: Option<String>,
    pub fingerprint: Option<String>,
    pub parameters: Vec<SqlParameterResponse>,
    pub timings: QueryTimingResponse,
    pub rows: Option<RowsSummaryResponse>,
    pub error: Option<ErrorSummaryResponse>,
    pub metadata: ProtocolMetadataResponse,
}
```

Parameter values use explicit type/value JSON:

```json
{
  "index": 0,
  "name": "id",
  "value": {
    "type": "integer",
    "value": 42
  },
  "redacted": false
}
```

### 3. Contracts

- The route path is `/api/v1/sql-events/{id}`.
- The handler wraps `{id}` as `SqlEventId` and calls `RingBufferStore::get`.
- Existing retained event returns HTTP 200 with `SqlEventDetailResponse`.
- Missing event returns HTTP 404.
- Missing event uses `ApiErrorCode::NotFound` serialized as `NOT_FOUND`.
- Missing event error details include the requested ID as `details.id`.
- Detail response includes all list-summary fields plus `normalized_sql`, `parameters`, `timings`, and `error`.
- Detail response reuses lowercase/snake_case status and kind mappings.
- Detail response reuses protocol-keyed metadata mapping.
- Request ID middleware applies to success and error responses.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| Retained event exists for `{id}` | Return HTTP 200 detail response |
| Event is missing or evicted | Return HTTP 404 `NOT_FOUND` |
| Event has parameters | Return typed parameter values |
| Event has error summary | Return error code, SQL state, message, and protocol metadata |
| Event has no error summary | Return `error: null` |
| Request omits `x-request-id` | Response includes a generated `x-request-id` header |

### 5. Good/Base/Bad Cases

Good:

- Detail endpoint tests inject retained events through `router_with_state`.
- Tests assert parameters, timings, rows, error, and metadata.
- Missing-event tests assert both HTTP 404 and `NOT_FOUND`.

Base:

- A selected event from the list endpoint can be fetched by its `id` without translation.

Bad:

- Returning HTTP 200 with `null` for a missing event.
- Returning core `SqlEvent` directly and leaking Rust enum variant names.
- Building a separate storage lookup abstraction before a second backend exists.

### 6. Tests Required

For SQL event detail endpoint changes:

- Existing event HTTP 200 test.
- Missing event HTTP 404 test.
- Missing event `NOT_FOUND` error body test.
- Detail payload test covering parameters, timings, rows, error summary, and metadata.
- Existing SQL event list endpoint tests still pass.
- Run `cargo fmt --check`.
- Run `cargo check --workspace`.
- Run `cargo test --workspace`.
- Run `cargo clippy --workspace --all-targets -- -D warnings`.

### 7. Wrong vs Correct

#### Wrong

```rust
async fn get_sql_event_detail(Path(id): Path<String>) -> Json<Option<SqlEvent>> {
    Json(store.get(&SqlEventId(id)).cloned())
}
```

#### Correct

```rust
async fn get_sql_event_detail(
    Extension(state): Extension<ApiState>,
    Path(id): Path<String>,
) -> Result<Json<SqlEventDetailResponse>, ApiEndpointError> {
    let event = {
        let event_store = state.event_store();
        let store = event_store.read().await;
        store.get(&SqlEventId(id.clone())).cloned()
    }
    .ok_or_else(|| ApiEndpointError::not_found("SQL event not found", "id", id))?;

    Ok(Json(SqlEventDetailResponse::from(&event)))
}
```

## Scenario: Connections Endpoint Contracts

### 1. Scope / Trigger

- Trigger: `sql-lens-api` exposes retained connection state through `GET /api/v1/connections` and `GET /api/v1/connections/{id}`.
- The endpoint is API-facing and must format `ConnectionInfo` into `API.md` JSON shape.
- It must not wire live proxy runtime updates; runtime composition belongs to a later task.

### 2. Signatures

Endpoints:

```http
GET /api/v1/connections
GET /api/v1/connections/{id}
```

Public API constants and DTOs:

```rust
pub const CONNECTIONS_PATH: &str = "/api/v1/connections";
pub const CONNECTION_DETAIL_PATH: &str = "/api/v1/connections/{id}";

pub struct ConnectionListResponse {
    pub items: Vec<ConnectionResponse>,
}

pub struct ConnectionResponse {
    pub id: String,
    pub protocol: String,
    pub database_type: String,
    pub client_addr: String,
    pub backend_addr: String,
    pub user: Option<String>,
    pub database: Option<String>,
    pub state: String,
    pub connected_at: String,
    pub closed_at: Option<String>,
    pub last_activity_at: Option<String>,
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub query_count: u64,
}
```

API state:

```rust
pub const DEFAULT_CONNECTION_STORE_CAPACITY: usize = 10_000;

impl ApiState {
    pub fn with_stores(
        event_store: sql_lens_storage::RingBufferStore,
        connection_store: sql_lens_storage::ConnectionStore,
    ) -> Self;

    pub fn connection_store(&self) -> std::sync::Arc<tokio::sync::RwLock<sql_lens_storage::ConnectionStore>>;
}
```

### 3. Contracts

- `ApiState::new(event_store)` remains valid and creates a default connection store.
- `router_with_state` registers both connection routes under request ID middleware.
- `GET /api/v1/connections` returns `ConnectionListResponse`.
- List endpoint supports only `limit` in this task.
- Default `limit` is `100`.
- Maximum `limit` is `500`; larger values are clamped to `500`.
- `limit = 0` returns HTTP 400.
- List ordering is newest-updated first, inherited from `ConnectionStore::list_recent`.
- `GET /api/v1/connections/{id}` returns `ConnectionResponse` for retained connections.
- Missing or evicted connection returns HTTP 404 `NOT_FOUND`.
- `ConnectionState` values serialize as snake_case strings.
- Request ID middleware applies to success and error responses.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| No retained connections | Return HTTP 200 with empty `items` |
| Active and closed connections exist | Return both in newest-updated order |
| `limit = 0` | Return HTTP 400 `BAD_REQUEST` with `field = "limit"` |
| Existing connection detail requested | Return HTTP 200 with `ConnectionResponse` |
| Missing connection detail requested | Return HTTP 404 `NOT_FOUND` with `details.id` |
| Request omits `x-request-id` | Response includes generated `x-request-id` |

### 5. Good/Base/Bad Cases

Good:

- API tests inject a populated `ConnectionStore` via `ApiState::with_stores`.
- State strings are asserted as `ready`, `closed`, and similar snake_case values.

Base:

- Future proxy runtime code upserts connection lifecycle state into the same store.

Bad:

- Returning core `ConnectionInfo` directly and exposing Rust enum names.
- Adding cursor/filter support before a product task requires it.
- Starting proxy or app runtime just to test connection handlers.

### 6. Tests Required

For connections endpoint changes:

- List response includes active and closed connections.
- Detail response returns existing connection.
- Missing detail returns HTTP 404 and `NOT_FOUND`.
- Invalid list limit returns HTTP 400.
- Response includes request ID header.
- Existing health and SQL event endpoint tests still pass.
- Run `cargo fmt --check`.
- Run `cargo check --workspace`.
- Run `cargo test --workspace`.
- Run `cargo clippy --workspace --all-targets -- -D warnings`.

### 7. Wrong vs Correct

#### Wrong

```rust
async fn list_connections() -> Json<Vec<ConnectionInfo>> {
    Json(proxy.active_connections())
}
```

#### Correct

```rust
async fn list_connections(
    Extension(state): Extension<ApiState>,
    Query(params): Query<ConnectionListQueryParams>,
) -> Result<Json<ConnectionListResponse>, ApiEndpointError> {
    let limit = parse_limit(params.limit)?;
    let connections = {
        let connection_store = state.connection_store();
        let store = connection_store.read().await;
        store.list_recent(limit)
    };

    Ok(Json(ConnectionListResponse {
        items: connections.iter().map(ConnectionResponse::from).collect(),
    }))
}
```

## Scenario: Protocols Endpoint Contracts

### 1. Scope / Trigger

- Trigger: `sql-lens-api` exposes protocol discovery through `GET /api/v1/protocols`.
- The endpoint is API-facing and returns protocol-neutral capability metadata for UI and tooling.
- The first implementation is static. Dynamic adapter registry inspection belongs to a later runtime composition task.

### 2. Signatures

Endpoint:

```http
GET /api/v1/protocols
```

Public API constants and DTOs:

```rust
pub const PROTOCOLS_PATH: &str = "/api/v1/protocols";

pub struct ProtocolListResponse {
    pub items: Vec<ProtocolResponse>,
}

pub struct ProtocolResponse {
    pub name: String,
    pub status: String,
    pub databases: Vec<String>,
}
```

### 3. Contracts

- `router_with_state` registers the protocols route under request ID middleware.
- `GET /api/v1/protocols` returns `ProtocolListResponse`.
- `mysql` is listed with `status = "supported"`.
- The MySQL-compatible `databases` list contains `mysql`, `starrocks`, `tidb`, and `doris`.
- Planned protocol families use `status = "planned"`.
- Planned protocols may include `postgresql`, `clickhouse`, and `sqlite`.
- Response fields stay protocol-neutral; do not expose adapter-specific Rust types.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| Protocols endpoint requested | Return HTTP 200 with an `items` array |
| Request omits `x-request-id` | Response includes generated `x-request-id` |
| MySQL adapter is the only supported first protocol family | Return `mysql` as supported and other roadmap families as planned |

### 5. Good/Base/Bad Cases

Good:

- A static response keeps early UI code from hard-coding protocol roadmap data.
- Future runtime code can replace or augment the source without changing the public DTO shape.

Base:

- Tests assert supported MySQL-compatible databases and at least representative planned protocols.

Bad:

- Reading protocol support from docs at runtime.
- Starting proxy or app runtime just to test protocol discovery.
- Adding MySQL-specific top-level fields to protocol discovery responses.

### 6. Tests Required

For protocols endpoint changes:

- Response includes request ID header.
- `mysql` is present with `supported` status.
- MySQL-compatible databases include `mysql`, `starrocks`, `tidb`, and `doris`.
- Planned protocols are present with `planned` status.
- Run `cargo fmt --check`.
- Run `cargo check --workspace`.
- Run `cargo test --workspace`.
- Run `cargo clippy --workspace --all-targets -- -D warnings`.

## Scenario: MySQL Client Command Observation

### 1. Scope / Trigger

- Trigger: `sql-lens-protocol-mysql` parses or observes MySQL client command bytes.
- Command parsing is protocol-local. Shared `SqlEvent`, storage, API, and UI contracts should change only when a task explicitly asks for a new captured event surface.
- Connection activity commands such as `COM_PING` and `COM_QUIT` are not SQL statements.

### 2. Signatures

Parser contracts live in:

```rust
pub fn parse_client_command(
    payload: &[u8],
) -> Result<Option<MysqlParsedClientCommand>, MysqlCommandParseError>;

pub enum MysqlParsedClientCommand {
    Query(MysqlComQuery),
    Ping(MysqlComPing),
    StatementPrepare(MysqlComStmtPrepare),
    StatementExecute(MysqlComStmtExecute),
    StatementClose(MysqlComStmtClose),
    Quit(MysqlComQuit),
}
```

Adapter state exposes read-only connection state for focused tests:

```rust
impl MysqlConnectionState {
    pub fn connection(&self) -> &ConnectionInfo;
}
```

### 3. Contracts

- Unsupported command bytes return `Ok(None)` and remain non-fatal.
- Malformed supported commands return `MysqlCommandParseError`; adapter observation treats parse failure as non-fatal.
- `COM_QUERY` starts a pending SQL query and may emit a `SqlEvent` after a terminal backend response.
- `COM_PING` updates `ConnectionInfo.last_activity_at` using the observation clock and must not start a pending query or emit a SQL event.
- `COM_QUIT` updates `ConnectionInfo.last_activity_at`, sets `ConnectionInfo.state` to `ConnectionState::Closing`, and must not start a pending query or emit a SQL event.
- Prepared statement lifecycle commands mutate only MySQL-local prepared statement state unless a later task adds a public event contract.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| Empty command payload | Return `IncompletePayload { field: "command" }` |
| Unsupported command byte | Return `Ok(None)` |
| `COM_PING` after authentication | Store last command, update `last_activity_at`, emit no events |
| `COM_QUIT` after authentication | Store last command, update `last_activity_at`, mark connection `Closing`, emit no events |
| Ping or quit before authentication | Ignore command-specific observation |
| Ping or quit with backend OK packet | Do not create or finalize a SQL event |

### 5. Good/Base/Bad Cases

Good:

- Adding a new MySQL command starts with parser tests plus adapter state tests.
- Connection-activity commands use empty SQL strings only in MySQL-local `MysqlClientCommand`, never in shared `SqlEvent`.

Base:

- `COM_QUERY`, `COM_STMT_PREPARE`, `COM_STMT_EXECUTE`, and `COM_STMT_CLOSE` keep their existing behavior after adding a command variant.

Bad:

- Storing `COM_PING` as `SqlEventKind::Query`.
- Incrementing query counters or mutating prepared statement maps for ping or quit.
- Adding MySQL-only command fields to protocol-neutral core structs.

### 6. Tests Required

For MySQL command observation changes:

- Parser unit tests for each new supported command byte.
- Adapter tests for authenticated-state behavior.
- Regression tests proving non-SQL commands do not create `pending_query` or emit `SqlEvent`s.
- Existing query, prepare, execute, and close tests continue to pass.
- Run `cargo fmt --check`.
- Run `cargo test -p sql-lens-protocol-mysql`.
- Run `cargo test --workspace`.
- Run `cargo clippy --workspace --all-targets -- -D warnings`.

### 7. Wrong vs Correct

#### Wrong

```rust
MysqlParsedClientCommand::Ping(_) => {
    self.pending_query = Some(MysqlPendingQuery { ... });
}
```

#### Correct

```rust
MysqlParsedClientCommand::Ping(_) => {
    let time = clock.now();
    self.connection.last_activity_at = Some(time.timestamp);
}
```

## Scenario: SQL Fingerprinting Foundation

### 1. Scope / Trigger

- Trigger: backend code needs a stable grouping key for similar SQL text.
- Fingerprinting is protocol-neutral core behavior. Protocol adapters may call
  it when building `SqlEvent`, but they must not duplicate local normalization
  algorithms.
- The first foundation is scanner-based, not an AST parser or dialect-specific
  normalizer.

### 2. Signatures

The shared helper lives in `sql-lens-core` and is re-exported from `lib.rs`:

```rust
pub fn fingerprint_sql(sql: &str) -> String;
```

Adapter event construction should populate the existing event field:

```rust
SqlEvent {
    fingerprint: Some(fingerprint_sql(sql_for_grouping)),
    ..
}
```

### 3. Contracts

- `fingerprint_sql` is total over arbitrary text and must not return `Result`.
- Common string, numeric, hexadecimal, `NULL`, `TRUE`, and `FALSE` literals are
  replaced with `?`.
- ASCII whitespace is collapsed, and punctuation/comparison spacing is
  normalized so `id=42` and `id = 42` group together.
- The helper preserves statement shape, identifiers, punctuation, and operators
  well enough for debug grouping.
- `COM_QUERY` events fingerprint the original query SQL.
- Prepared execute events prefer expanded SQL for fingerprinting when available,
  then fall back to the prepared template SQL.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| Empty SQL text | Return an empty fingerprint |
| Malformed or unterminated string literal | Return a best-effort fingerprint, no panic |
| Literal value appears in supported form | Replace it with `?` |
| Identifier contains digits | Preserve it as an identifier, not a literal |
| Adapter cannot render expanded prepared SQL | Fingerprint the template SQL |

### 5. Good/Base/Bad Cases

Good:

- `SELECT * FROM users WHERE id = 42` and
  `select * from users where id=7` both group to the same fingerprint.
- A new protocol adapter calls `sql_lens_core::fingerprint_sql` instead of
  adding a private normalizer.

Base:

- The helper is intentionally parser-light; full dialect AST normalization can
  be added later behind the same public contract if product scope requires it.

Bad:

- Adding `serde_json`, SQL parser crates, runtime crates, or protocol-specific
  dependencies to `sql-lens-core` for the foundation implementation.
- Treating fingerprinting as redaction. Storage/API redaction remains the owner
  of masking sensitive values.

### 6. Tests Required

- Core unit tests for common `SELECT`, `INSERT`, `UPDATE`, and `DELETE`
  statements.
- Core tests for whitespace, punctuation/operator spacing, quoted strings, and
  identifiers containing digits.
- Protocol adapter tests proving emitted SQL events populate
  `SqlEvent.fingerprint`.
- Run `cargo fmt --check`.
- Run `cargo test -p sql-lens-core`.
- Run `cargo test -p sql-lens-protocol-mysql` when adapter wiring changes.
- Run `cargo test --workspace`.
- Run `cargo clippy --workspace --all-targets -- -D warnings`.

### 7. Wrong vs Correct

#### Wrong

```rust
SqlEvent {
    fingerprint: Some(mysql_only_fingerprint(&sql)),
    ..
}
```

#### Correct

```rust
SqlEvent {
    fingerprint: Some(sql_lens_core::fingerprint_sql(&sql)),
    ..
}
```

## Scenario: Plugin Hook Trait Contracts

### 1. Scope / Trigger

- Trigger: `sql-lens-plugin` exposes public in-process extension points for
  connection, query, prepared statement, execution, and error observation.
- These contracts are cross-crate boundaries for future runtime dispatchers and
  exporters. They must stay protocol-neutral and must not affect packet
  forwarding.

### 2. Signatures

Plugin hooks live in `crates/sql-lens-plugin/src/lib.rs` and use only
`sql-lens-core` payload models:

```rust
pub type PluginResult = Result<(), PluginError>;

pub trait OnConnect {
    fn on_connect(&mut self, connection: &ConnectionInfo) -> PluginResult;
}

pub trait OnQuery {
    fn on_query(&mut self, event: &SqlEvent, connection: &ConnectionInfo) -> PluginResult;
}
```

`OnPrepare`, `OnExecute`, and `OnError` follow the same synchronous,
object-safe pattern using `PreparedStatementInfo`, `SqlEvent`, `ConnectionInfo`,
and `ErrorSummary` as appropriate.

### 3. Contracts

- `OnConnect` receives `ConnectionInfo`; it already includes protocol,
  database type, client address, and backend address.
- `OnQuery` and `OnExecute` receive `SqlEvent`; its metadata, parameters, and
  expanded SQL remain protocol-neutral shared data.
- `OnPrepare` receives `PreparedStatementInfo`, not a protocol-local statement
  ID type.
- `OnError` receives both `SqlEvent` and `ErrorSummary` for direct error
  handling without requiring consumers to inspect optional event fields.
- Callback methods are synchronous, borrowed, and object-safe. They return
  `PluginResult`; no runtime scheduling, timeout, retry, loading, or dispatch
  behavior belongs in this crate.
- Dispatchers must pass redacted `SqlEvent` values before hook invocation.
- `PluginError` crosses the plugin boundary as a typed error, but a returned
  error must never stop packet forwarding or captured-event delivery.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| Hook succeeds | Return `Ok(())` |
| Hook cannot complete | Return `Err(PluginError::HookFailed { .. })` |
| Dispatcher receives a plugin error | Record or isolate it; continue forwarding and capture delivery |
| Hook payload needs protocol-specific data | Read it through `ProtocolMetadata`, not a new MySQL-only field |
| SQL event reaches a hook | Dispatch only an already-redacted event |

### 5. Good/Base/Bad Cases

Good:

- A webhook exporter implements only `OnError` and returns `PluginResult`.
- A future dispatcher keeps `Box<dyn OnQuery>` values and handles each returned
  error independently.

Base:

- A stateful plugin uses `&mut self` to count received events.

Bad:

- Putting `MysqlPreparedStatement` or raw packet bytes in a plugin trait.
- Adding async runtime or HTTP dependencies solely to define hook traits.
- Letting a plugin error terminate a proxy session.

### 6. Tests Required

- Construct representative `ConnectionInfo`, `SqlEvent`,
  `PreparedStatementInfo`, and `ErrorSummary` payloads.
- Invoke every hook through a trait object to prove object safety.
- Assert successful callbacks receive the expected protocol-neutral fields.
- Assert a failing hook returns `PluginError` and that it implements
  `std::error::Error`.
- Run `cargo fmt --check`, `cargo test -p sql-lens-plugin`, workspace tests,
  and workspace clippy.

### 7. Wrong vs Correct

#### Wrong

```rust
pub trait OnQuery {
    fn on_query(&mut self, statement_id: u32, raw_packet: &[u8]);
}
```

#### Correct

```rust
pub trait OnQuery {
    fn on_query(&mut self, event: &SqlEvent, connection: &ConnectionInfo) -> PluginResult;
}
```

## Forbidden Patterns

- Do not put MySQL-only fields directly on shared core models.
- Do not add arbitrary JSON metadata without a task-level design update.
- Do not add runtime, HTTP, database, or storage dependencies to `sql-lens-core` for model-only work.
- Do not leave new public models without construction tests.

## Code Review Checklist

- Does the change preserve protocol neutrality?
- Are new public types re-exported from `lib.rs`?
- Are dependencies still minimal?
- Are serde traits available where required?
- Is `Eq` only derived where all fields can support it?
- Do tests cover representative construction and trait availability?
