# TCP Proxy Listener Implementation Plan

## Preconditions

- Planning artifacts are reviewed.
- Task is activated with `task.py start`.
- Implementation remains limited to the `sql-lens-proxy` listener boundary.

## Files To Modify

- `crates/sql-lens-proxy/Cargo.toml`
- `crates/sql-lens-proxy/src/lib.rs`
- `Cargo.lock`
- `.trellis/spec/backend/quality-guidelines.md`

## Checklist

1. [x] Add `tokio` dependency to `sql-lens-proxy` with `net`, `sync`, `time`, `rt`, and `macros` features.
2. [x] Add `tracing` dependency to `sql-lens-proxy`.
3. [x] Define `ProxyListenerConfig`.
4. [x] Define `TcpProxyListener`.
5. [x] Implement `TcpProxyListener::bind`.
6. [x] Implement `TcpProxyListener::local_addr`.
7. [x] Define `AcceptedClient` with peer address and owned stream.
8. [x] Implement `TcpProxyListener::accept`.
9. [x] Define `AcceptLoopStats`.
10. [x] Define `ProxyListenerError` with structured variants.
11. [x] Implement `Display` and `std::error::Error` for `ProxyListenerError`.
12. [x] Implement `run_accept_loop` with shutdown support.
13. [x] Add async test for successful bind.
14. [x] Add async test for structured bind failure.
15. [x] Add async test for accepting one client connection.
16. [x] Add async test for shutdown without connection.
17. [x] Update backend spec with the listener contract.
18. [x] Run validation.
19. [x] Verify no out-of-scope proxy behavior was introduced.

## Validation Results

- [x] `rtk cargo fmt --check`
- [x] `rtk cargo check --workspace`
- [x] `rtk cargo test --workspace`
- [x] `rtk cargo clippy --workspace --all-targets -- -D warnings`
- [x] `rtk cargo tree -p sql-lens-proxy`
- [x] `rtk rg -n "connect\\(|copy_bidirectional|AsyncReadExt|AsyncWriteExt|sql-lens-app|sql_lens_config|protocol|capture|backend" crates/sql-lens-proxy`
- [x] `rtk python3 ./.trellis/scripts/task.py validate .trellis/tasks/07-06-add-tcp-proxy-listener`

Note: socket-binding tests require an environment that permits local TCP binds. The sandbox denied `127.0.0.1:0` with `Operation not permitted`, so socket tests were rerun with approved elevated execution.

## Validation Commands

```bash
rtk cargo fmt --check
rtk cargo check --workspace
rtk cargo test --workspace
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk cargo tree -p sql-lens-proxy
rtk rg -n "connect\\(|copy_bidirectional|AsyncReadExt|AsyncWriteExt|sql-lens-app|sql_lens_config|protocol|capture|backend" crates/sql-lens-proxy
rtk python3 ./.trellis/scripts/task.py validate .trellis/tasks/07-06-add-tcp-proxy-listener
```

## Review Gate

Do not implement:

- Backend dialing.
- Client/backend byte forwarding.
- Connection IDs.
- Connection lifecycle records.
- Protocol parsing.
- Capture pipeline.
- CLI runtime integration.
- Signal handling.
- TLS handling.
- Max connection enforcement.
- Idle timeout enforcement.

## Commit Plan

When implementation and validation pass, create one work commit:

```text
feat: add tcp proxy listener
```

Do not archive the task until the work commit exists.
