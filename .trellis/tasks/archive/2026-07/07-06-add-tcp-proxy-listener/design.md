# TCP Proxy Listener Design

## Objective

Create the first usable network primitive in `sql-lens-proxy`:

```text
ProxyListenerConfig
  -> TcpProxyListener::bind(...)
  -> TcpProxyListener::run_accept_loop(...)
  -> AcceptedClient channel
  -> Shutdown stops accepting
```

This task is library-level proxy work. It should not make the CLI process long-running yet.

## Crate Boundary

Modify:

- `crates/sql-lens-proxy/Cargo.toml`
- `crates/sql-lens-proxy/src/lib.rs`
- `Cargo.lock`

Likely spec update:

- `.trellis/spec/backend/quality-guidelines.md`
- Optional: `.trellis/spec/backend/directory-structure.md` if proxy ownership needs more detail.

Task metadata lives under:

- `.trellis/tasks/07-06-add-tcp-proxy-listener/`

Do not modify:

- `crates/sql-lens-app/src/main.rs`
- `crates/sql-lens-config/src/lib.rs`
- Protocol, storage, API, or frontend crates

## Dependencies

Add to `sql-lens-proxy`:

```toml
tokio = { version = "1", features = ["net", "sync", "time", "rt", "macros"] }
tracing = "0.1"
```

Rationale:

- `tokio::net::TcpListener` owns non-blocking listener bind and accept.
- `tokio::sync::watch` or equivalent shutdown receiver supports stopping the accept loop.
- `tokio::sync::mpsc` provides a small boundary between accepted sockets and future session/backend dialing work.
- `tokio::time::timeout` supports deterministic async tests.
- `tracing` can emit low-sensitivity lifecycle events such as listener bind and shutdown.

Avoid `tokio-util::sync::CancellationToken` for this first listener. A `watch::Receiver<bool>` is sufficient and avoids an extra dependency.

## Public API Shape

Recommended API:

```rust
pub struct ProxyListenerConfig {
    pub listen: String,
}

pub struct TcpProxyListener { ... }

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

pub struct AcceptedClient { ... }

impl AcceptedClient {
    pub fn peer_addr(&self) -> std::net::SocketAddr;
    pub fn into_stream(self) -> tokio::net::TcpStream;
}

pub struct AcceptLoopStats {
    pub accepted_connections: u64,
}

pub enum ProxyListenerError {
    Bind { listen: String, source: std::io::Error },
    LocalAddr { source: std::io::Error },
    Accept { source: std::io::Error },
    AcceptedClientReceiverClosed,
}
```

The exact names may vary slightly during implementation, but these contracts should stay stable.

## Data Flow

```text
TcpProxyListener::bind(config.listen)
  -> Tokio TcpListener
  -> run_accept_loop
       select! accept() or shutdown.changed()
       accept() success -> AcceptedClient -> mpsc Sender
       shutdown true / sender dropped -> exit with stats or structured error
```

Accepted sockets are not connected to backends yet. The accept loop only hands them to a channel for future session logic. Tests can receive the accepted client and drop it.

## Shutdown Contract

- `shutdown` starts as `false`.
- Sending `true` stops the accept loop.
- Dropping the shutdown sender should also stop the accept loop, because there is no future controller.
- Shutdown stops accepting new client sockets.
- This task does not drain active sessions; Issue 016 owns that.

## Error Contract

- Bind error includes the requested listen string and the source `std::io::Error`.
- Local address lookup error preserves the source `std::io::Error`.
- Accept error preserves the source `std::io::Error`.
- If the accepted-client receiver is closed, return a structured receiver-closed error.
- `ProxyListenerError` implements `Debug`, `Display`, and `std::error::Error`.

## Test Strategy

Use async unit tests inside `sql-lens-proxy`.

Recommended tests:

- `listener_binds_configured_address`:
  - bind `127.0.0.1:0`
  - assert local port is non-zero
  - assert loopback address
- `bind_failure_returns_structured_error`:
  - bind `127.0.0.1:0`
  - bind a second listener to the first listener's local address
  - assert `ProxyListenerError::Bind { listen, .. }`
- `accept_loop_delivers_client_connection`:
  - bind `127.0.0.1:0`
  - run accept loop with `mpsc`
  - connect a `tokio::net::TcpStream`
  - assert received peer address exists
  - send shutdown and assert accepted count
- `accept_loop_stops_on_shutdown`:
  - start accept loop
  - send shutdown without connecting
  - assert loop exits with zero accepted count

Use `tokio::time::timeout` to prevent hanging tests.

## Compatibility

- Existing CLI behavior remains unchanged and still exits after config validation/logging.
- `sql-lens-config` remains free of socket bind probing.
- Future Issue 014 can consume `AcceptedClient` and perform backend dialing.
- Future Issue 016 can replace or extend shutdown coordination for active session draining.

## Risks

- Listener tests can be flaky if they use fixed ports. Use `127.0.0.1:0` except for intentional second-bind failure against an already-bound local address.
- Accept loop tests can hang if shutdown is not selected. Always use `timeout`.
- Sending raw `TcpStream` through a channel exposes ownership decisions early. Keep `AcceptedClient` minimal and avoid session IDs until connection lifecycle work.

## Rollback

Rollback by removing:

- `tokio` and `tracing` dependencies from `sql-lens-proxy`.
- Listener structs, errors, and tests from `sql-lens-proxy`.
- Lockfile entries introduced only by this task.
