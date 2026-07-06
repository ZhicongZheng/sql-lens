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

## Scenario: Ring Buffer Append Contracts

### 1. Scope / Trigger

- Trigger: `sql-lens-storage` owns the default in-memory storage backend for retained SQL events.
- Ring buffer append is the first storage primitive and must stay append-oriented.
- This layer bounds memory by capacity and evicts oldest events first.

### 2. Signatures

Public ring buffer types live in `crates/sql-lens-storage/src/lib.rs`:

```rust
pub struct RingBufferStore;

impl RingBufferStore {
    pub fn new(capacity: std::num::NonZeroUsize) -> Self;
    pub fn append(&mut self, event: sql_lens_core::SqlEvent) -> RingBufferAppendOutcome;
    pub fn get(&self, id: &sql_lens_core::SqlEventId) -> Option<&sql_lens_core::SqlEvent>;
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
- Timeline pagination, filters, retention, and secondary indexes belong to later storage tasks.

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

### 5. Good/Base/Bad Cases

Good:

- Ring buffer append stays synchronous and in-memory.
- Tests use synthetic `SqlEvent` values from `sql-lens-core`.

Base:

- Future performance work may add an ID index while preserving append and eviction semantics.
- Future retention work may add age/byte eviction after this oldest-first baseline.

Bad:

- Adding secondary indexes before query behavior proves they are needed.
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
