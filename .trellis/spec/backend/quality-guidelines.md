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

- Trigger: `sql-lens-config` owns startup configuration structs, serde-compatible config shape, default values, and startup TOML parsing.
- Config loading is a boundary contract for CLI startup, validation, logging setup, runtime startup, and future hot reload.
- Config parsing must stay separate from semantic validation and runtime apply logic.

### 2. Signatures

Public TOML loading APIs live on `SqlLensConfig`:

```rust
impl SqlLensConfig {
    pub fn from_path(path: impl AsRef<std::path::Path>) -> Result<Self, ConfigLoadError>;
    pub fn from_toml_str(input: &str) -> Result<Self, ConfigLoadError>;
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
```

### 3. Contracts

- `from_path` reads a file and parses it as TOML.
- `from_toml_str` parses already-loaded TOML content.
- Missing sections and fields use the existing `Default` implementations.
- Unknown sections and fields are rejected with `#[serde(deny_unknown_fields)]`.
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
| Required semantic value is empty or unsupported at runtime | Leave to the later config validation layer |

### 5. Good/Base/Bad Cases

Good:

- A local config file contains only `[proxy] listen = "127.0.0.1:3308"` and the rest comes from defaults.
- A misspelled field like `lissten` fails during TOML deserialization.
- A missing file reports `ConfigLoadError::Read`.

Base:

- A caller uses `SqlLensConfig::from_toml_str` in tests and `SqlLensConfig::from_path` in CLI code.
- Validation later rejects semantically invalid values after TOML parsing succeeds.

Bad:

- Silently ignoring unknown config fields.
- Starting services from `sql-lens-config`.
- Adding environment overrides, hot reload, or CLI argument parsing inside `sql-lens-config`.

### 6. Tests Required

For config loading changes:

- Valid TOML file loads from a path.
- TOML string parsing works.
- Partial TOML falls back to defaults.
- Unknown top-level sections and nested fields fail.
- Invalid TOML returns `ConfigLoadError::Parse`.
- Missing files return `ConfigLoadError::Read`.
- `ConfigLoadError` implements `Display` and `std::error::Error`.

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
- The first CLI layer is a startup contract for local development, CI smoke tests, and future runtime composition.
- Keep this layer synchronous until a runtime startup task explicitly adds async services.

### 2. Signatures

The initial command surface is:

```text
sql-lens [--config <FILE>]
sql-lens --version
sql-lens --help
```

The Rust entry point shape should stay small:

```rust
fn main() -> std::process::ExitCode;
fn run(cli: Cli) -> Result<(), AppError>;
```

Allowed application startup dependencies in `sql-lens-app` at this stage:

```toml
clap = { version = "4", features = ["derive"] }
sql-lens-config = { path = "../sql-lens-config" }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["json"] }
```

### 3. Contracts

- `--config <FILE>` loads the selected TOML file through `SqlLensConfig::from_path`.
- The default config path is `sql-lens.toml`.
- The loaded config is validated through `SqlLensConfig::validate`.
- `--version` is handled by clap and exits successfully without loading config.
- Successful load, validation, and logging initialization exit with `ExitCode::SUCCESS`.
- Config load or validation failure prints a human-readable message to stderr and exits with `ExitCode::FAILURE`.
- Logging initialization happens after config validation; follow `logging-guidelines.md`.
- Do not start proxy, API, storage, signal handling, hot reload, or async runtime services in this layer.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| `--version` is passed | Print version and exit zero before config loading |
| `--config <FILE>` points to a valid config | Load, validate, initialize logging, emit startup-check log, exit zero |
| Config file cannot be read | Include config load context and the path in stderr; exit non-zero |
| TOML cannot be parsed | Include config load context and parse error in stderr; exit non-zero |
| Config validation fails | Include validation context and violation details in stderr; exit non-zero |
| Running without `--config` | Attempt to load `sql-lens.toml` |

### 5. Good/Base/Bad Cases

Good:

- CLI code delegates parsing to clap and delegates config semantics to `sql-lens-config`.
- App-level errors wrap config errors only to add startup context.

Base:

- Integration tests run the compiled `sql-lens` binary with standard library `Command`.
- Test configs use temporary files and explicit `--config` paths.

Bad:

- Duplicating config validation rules in `sql-lens-app`.
- Calling `unwrap` or `expect` on user-provided config load/validation paths.
- Adding async runtime, HTTP, storage, watcher, or service startup dependencies to satisfy CLI or logging startup tasks.

### 6. Tests Required

For CLI entry point changes:

- `--version` exits successfully and includes the package version.
- `--config <valid-file>` exits successfully.
- Missing config path exits non-zero and stderr includes load/read context.
- Invalid config exits non-zero and stderr includes validation context and violation fields.
- Run `cargo fmt --check`.
- Run `cargo check --workspace`.
- Run `cargo test --workspace`.
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
