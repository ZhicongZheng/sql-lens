# Proxy Graceful Shutdown Design

## Objective

Add the first proxy-local shutdown coordination layer:

```text
shutdown requested
  -> stop listener through existing watch<bool>
  -> notify active sessions
  -> drain active session tasks within timeout
  -> ShutdownDrainSummary
```

This task does not start the app runtime or listen for OS signals. It creates reusable proxy primitives that a later runtime composition task can call.

## Crate Boundary

Modify:

- `crates/sql-lens-config/src/lib.rs`
- `crates/sql-lens-proxy/src/lib.rs`
- `Cargo.lock` if dependency features change
- `.trellis/spec/backend/quality-guidelines.md`
- `CONFIG.md` only if needed to keep documented config keys aligned

Do not modify:

- `crates/sql-lens-app/src/main.rs`
- protocol crates
- storage crate
- API crate
- frontend files

## Config Contract

Add to `ProxyConfig`:

```rust
pub shutdown_timeout_ms: u64,
```

Recommended default:

```rust
shutdown_timeout_ms: 10_000
```

Rationale:

- It is long enough for local in-flight requests to finish.
- It is short enough that a stuck session does not block shutdown indefinitely.
- It is separate from `idle_timeout_ms`; idle timeout describes connection inactivity, shutdown timeout describes process/service drain.

## Proxy API Shape

Recommended API:

```rust
pub struct ProxyShutdownConfig {
    pub drain_timeout: Duration,
}

impl ProxyShutdownConfig {
    pub fn new(drain_timeout: Duration) -> Self;
    pub fn from_config(proxy: &ProxyConfig) -> Self;
}

pub struct ProxyShutdownSignal {
    sender: watch::Sender<bool>,
}

impl ProxyShutdownSignal {
    pub fn new() -> Self;
    pub fn subscribe(&self) -> watch::Receiver<bool>;
    pub fn request_shutdown(&self) -> Result<(), ProxyShutdownError>;
}

pub struct ActiveSessionDrain;

impl ActiveSessionDrain {
    pub async fn drain<T>(
        sessions: Vec<tokio::task::JoinHandle<T>>,
        config: &ProxyShutdownConfig,
    ) -> ShutdownDrainSummary;
}
```

Names may vary, but keep these semantics:

- one signal fan-outs to listener and sessions,
- drain accepts already-spawned active session handles,
- drain timeout is bounded and structured.

## Session Notification Contract

Use `watch::Sender<bool>` / `watch::Receiver<bool>` for the first shutdown signal:

- `false` means running.
- `true` means shutdown requested.
- Listener already understands this shape.
- Sessions can receive a cloned receiver and select on `changed()`.

Do not introduce `tokio-util::sync::CancellationToken` yet. It is nice, but not needed for the first concrete implementation.

## Drain Contract

The drain operation should:

1. receive a collection of active session `JoinHandle`s,
2. wait for all handles to complete,
3. stop waiting after `shutdown_timeout_ms`,
4. abort unfinished handles after timeout,
5. return a summary.

Recommended summary:

```rust
pub struct ShutdownDrainSummary {
    pub completed_sessions: usize,
    pub failed_sessions: usize,
    pub timed_out_sessions: usize,
    pub timed_out: bool,
}
```

For this first layer, session output type does not need to be interpreted. Join success counts as completed. Join error counts as failed. Timeout counts remaining handles as timed out.

## Error Contract

Keep errors small and local:

- requesting shutdown can fail only if all receivers are gone,
- drain timeout is not an exception; it is represented in `ShutdownDrainSummary`.

## Test Strategy

Config tests in `sql-lens-config`:

- default config includes `shutdown_timeout_ms`.
- TOML override parses `shutdown_timeout_ms`.
- partial TOML still defaults it.
- unknown config fields still fail.

Proxy tests in `sql-lens-proxy`:

- shutdown signal starts as `false` and changes to `true`.
- `TcpProxyListener::run_accept_loop` still stops when signal changes.
- drain reports completed sessions when handles finish before timeout.
- drain reports timeout and aborts unfinished handles.
- session notification receiver observes shutdown.

Use `tokio::time::timeout` around async tests that could hang.

## Compatibility

- Existing listener API remains usable because it still accepts `watch::Receiver<bool>`.
- Future app runtime can own one `ProxyShutdownSignal`, pass receivers into listener and session tasks, and call drain during shutdown.
- Future connection lifecycle can convert drain summaries into lifecycle updates.

## Risks

- Aborting tasks can hide cleanup work. This task should only abort after the configured drain timeout.
- If drain generics become awkward, prefer a concrete `JoinHandle<()>` or `JoinHandle<Result<ForwardingSummary, ForwardingError>>` over a broad abstraction.
- Avoid pulling runtime startup concerns into proxy just to test shutdown.

## Rollback

Rollback by removing:

- `shutdown_timeout_ms` config field and tests,
- shutdown signal/drain structs and tests,
- config docs/spec updates introduced for this task.
