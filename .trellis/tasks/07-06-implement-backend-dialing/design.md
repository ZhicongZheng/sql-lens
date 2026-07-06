# Backend Dialing Design

## Objective

Extend `sql-lens-proxy` from accepting client sockets to creating the upstream backend socket:

```text
AcceptedClient
  + BackendDialConfig
  -> BackendDialer::dial(...)
  -> ProxiedConnection | BackendDialError
```

This task stops before byte forwarding. A successful result only pairs streams for the next task.

## Crate Boundary

Modify:

- `crates/sql-lens-proxy/Cargo.toml`
- `crates/sql-lens-proxy/src/lib.rs`
- `Cargo.lock`

Likely spec update:

- `.trellis/spec/backend/quality-guidelines.md`

Do not modify:

- `crates/sql-lens-app/src/main.rs`
- protocol crates
- storage crate
- API crate
- frontend files

## Dependencies

Existing `sql-lens-proxy` dependencies:

```toml
tokio = { version = "1", features = ["net", "sync", "time", "rt", "macros"] }
tracing = "0.1"
```

Add only if needed to satisfy config sourcing:

```toml
sql-lens-config = { path = "../sql-lens-config" }
```

Rationale:

- `BackendConfig.address` is the configured backend address.
- `ProxyConfig.connect_timeout_ms` is the configured dial timeout.
- Keeping the conversion in `sql-lens-proxy` lets tests verify the acceptance criterion without starting `sql-lens-app`.

## Public API Shape

Recommended API:

```rust
pub struct BackendDialConfig {
    pub address: String,
    pub connect_timeout: std::time::Duration,
}

impl BackendDialConfig {
    pub fn new(address: impl Into<String>, connect_timeout: Duration) -> Self;
    pub fn from_config(proxy: &ProxyConfig, backend: &BackendConfig) -> Self;
}

pub struct BackendDialer;

impl BackendDialer {
    pub async fn dial(
        accepted: AcceptedClient,
        config: &BackendDialConfig,
    ) -> Result<ProxiedConnection, BackendDialError>;
}

pub struct ProxiedConnection { ... }
```

Failure contract:

```rust
pub struct BackendDialFailure {
    pub client_peer_addr: SocketAddr,
    pub backend_address: String,
    pub kind: BackendDialFailureKind,
}

pub enum BackendDialFailureKind {
    Timeout { timeout: Duration },
    Connect,
}

pub enum BackendDialError {
    Timeout { failure: BackendDialFailure },
    Connect {
        failure: BackendDialFailure,
        source: std::io::Error,
    },
}
```

Names may vary slightly during implementation, but the behavior should remain stable.

## Data Flow

```text
AcceptedClient(peer_addr, client_stream)
  -> BackendDialer::dial
       timeout(connect_timeout, TcpStream::connect(address))
         success -> ProxiedConnection(client_stream, backend_stream, peer, backend)
         elapsed -> BackendDialError::Timeout + failure record
         io error -> BackendDialError::Connect + failure record
```

The accepted client is consumed by dialing. On failure, the connection is not recoverable in this first design; later lifecycle code can record the failure and close the client stream by dropping it.

## Timeout Contract

- `connect_timeout_ms` maps to `Duration::from_millis`.
- Timeout should wrap the entire `TcpStream::connect` future.
- If timeout elapses, return a structured timeout error.
- Zero timeout should be treated as a valid immediate timeout behavior unless tests reveal Tokio cannot represent it cleanly. Do not add config validation here.

## Failure Record Contract

The failure record is deliberately lightweight and local to `sql-lens-proxy`:

- It is not a durable connection lifecycle record.
- It does not allocate connection IDs.
- It does not depend on `sql-lens-core`.
- It preserves enough context for Issue 017 to map into lifecycle storage later.

## Test Strategy

Use async tests in `sql-lens-proxy`.

Recommended helpers:

- Create an accepted client using the existing `TcpProxyListener` and a local client connection.
- Create a local backend listener on `127.0.0.1:0` for successful backend dial.

Recommended tests:

- `backend_dial_config_uses_runtime_config`:
  - construct `ProxyConfig` and `BackendConfig`
  - assert backend address and timeout are mapped
- `backend_dial_succeeds`:
  - create accepted client
  - bind backend listener
  - dial backend
  - assert client peer and backend address are preserved
- `backend_dial_failure_is_structured`:
  - choose an unused local port by binding then dropping a listener
  - dial that address
  - assert `BackendDialError::Connect` and failure record fields
- `backend_dial_timeout_is_enforced`:
  - use an address that deterministically does not complete within a short timeout if practical
  - if deterministic timeout is not practical on local CI, document the limitation and test timeout mapping instead

Use `tokio::time::timeout` around tests that can hang.

## Compatibility

- Existing listener API remains usable.
- Existing CLI behavior remains unchanged.
- Future Issue 015 can take `ProxiedConnection` and implement byte forwarding.
- Future Issue 017 can consume `BackendDialFailure` and convert it into connection lifecycle records.

## Risks

- Network failure behavior varies by OS. Prefer loopback refused connections for deterministic connect errors.
- Dial timeout tests can be flaky. Keep timeout tests deterministic or explicitly document the limitation.
- Adding a config dependency to proxy should remain limited to typed config conversion, not file loading, env overrides, or validation.

## Rollback

Rollback by removing:

- backend dial structs and errors from `sql-lens-proxy`
- config dependency if added only for this task
- backend dialing tests
- lockfile entries introduced only by this task
