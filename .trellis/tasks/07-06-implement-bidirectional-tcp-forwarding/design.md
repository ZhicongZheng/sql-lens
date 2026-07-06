# Bidirectional TCP Forwarding Design

## Objective

Extend `sql-lens-proxy` from paired sockets to raw transparent forwarding:

```text
ProxiedConnection
  -> TcpForwarder::forward(...)
  -> ForwardingSummary | ForwardingError
```

This task stops at byte movement and counters. It does not introduce session orchestration, protocol parsing, capture events, storage, graceful shutdown, or app runtime wiring.

## Crate Boundary

Modify:

- `crates/sql-lens-proxy/Cargo.toml`
- `crates/sql-lens-proxy/src/lib.rs`
- `Cargo.lock`
- `.trellis/spec/backend/quality-guidelines.md`

Do not modify:

- `crates/sql-lens-app/src/main.rs`
- protocol crates
- storage crate
- API crate
- frontend files

## Dependencies

Use Tokio `copy_bidirectional`, which requires `io-util`.

Expected dependency change:

```toml
tokio = { version = "1", features = ["net", "sync", "time", "rt", "macros", "io-util"] }
```

Do not add new crates. Do not introduce `tokio-util`; the first forwarding layer does not need cancellation tokens or codecs.

## Public API Shape

Recommended API:

```rust
pub struct TcpForwarder;

impl TcpForwarder {
    pub async fn forward(
        connection: ProxiedConnection,
    ) -> Result<ForwardingSummary, ForwardingError>;
}

pub struct ForwardingSummary {
    pub client_peer_addr: SocketAddr,
    pub backend_address: String,
    pub client_to_backend_bytes: u64,
    pub backend_to_client_bytes: u64,
}

pub enum ForwardingError {
    Io {
        failure: ForwardingFailure,
        source: std::io::Error,
    },
}

pub struct ForwardingFailure {
    pub client_peer_addr: SocketAddr,
    pub backend_address: String,
}
```

Names may vary slightly during implementation, but keep the behavior stable and proxy-local.

## Data Flow

```text
ProxiedConnection(client_stream, backend_stream, client_peer, backend_address)
  -> split into mutable client/backend streams inside forward()
  -> tokio::io::copy_bidirectional(&mut client_stream, &mut backend_stream)
       Ok((client_to_backend, backend_to_client))
         -> ForwardingSummary
       Err(source)
         -> ForwardingError::Io
```

Tokio documentation says the first count is bytes copied from the first stream to the second stream. Therefore pass client as `a` and backend as `b` so tuple order maps directly to:

- `client_to_backend_bytes`
- `backend_to_client_bytes`

## Close Contract

Tokio `copy_bidirectional` handles EOF by calling `shutdown()` on the opposite writer and continuing the other direction until it also shuts down. This task should rely on that behavior instead of hand-rolling two copy loops.

The clean completion condition is:

- `copy_bidirectional` returns `Ok((a_to_b, b_to_a))`.

The error condition is:

- `copy_bidirectional` returns `Err(source)`.

Do not add custom close semantics unless a test proves Tokio's default behavior is insufficient.

## Failure Contract

Forwarding errors preserve connection context:

- `client_peer_addr`
- `backend_address`
- source IO error

Do not invent durable connection lifecycle records in this task. Future Issue 017 can map the forwarding summary or failure into lifecycle storage.

## Test Strategy

Use async tests in `sql-lens-proxy`.

Recommended helper:

- Reuse the existing listener + backend listener setup from backend dialing tests to obtain a real `ProxiedConnection`.
- Spawn `TcpForwarder::forward(proxied)` in a task.
- Use client-side and backend-side streams to write/read bytes.

Recommended tests:

- `forwarding_copies_client_to_backend`:
  - write bytes from client side,
  - read exact bytes from backend side,
  - close both sides,
  - assert summary byte count.
- `forwarding_copies_backend_to_client`:
  - write bytes from backend side,
  - read exact bytes from client side,
  - close both sides,
  - assert summary byte count.
- `forwarding_reports_bidirectional_byte_counts`:
  - send bytes in both directions,
  - close both sides,
  - assert both counters.
- `forwarding_finishes_when_one_side_closes`:
  - shutdown one side after writing,
  - assert forward task completes.

Use `tokio::time::timeout` around forwarding tasks to avoid hanging tests.

## Compatibility

- Existing listener and backend dialing APIs remain usable.
- Future app runtime work can compose listener -> dialer -> forwarder.
- Future protocol capture work may observe or wrap streams later, but this task should keep the simple TCP copy boundary.

## Risks

- Full-duplex socket tests can hang if either side is not shut down. Use explicit shutdown and test timeouts.
- Byte count order can be accidentally reversed. Keep client stream as the first `copy_bidirectional` argument and test both directions.
- Adding `io-util` expands Tokio features for `sql-lens-proxy`; do not add more Tokio features without need.

## Rollback

Rollback by removing:

- forwarding structs and errors from `sql-lens-proxy`
- `io-util` Tokio feature if introduced only for this task
- forwarding tests
- backend forwarding spec section
