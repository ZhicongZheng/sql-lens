# Backend Dialing Implementation Plan

## Preconditions

- Planning artifacts are reviewed.
- Task is activated with `task.py start`.
- Implementation remains limited to backend dialing in `sql-lens-proxy`.

## Files To Modify

- `crates/sql-lens-proxy/Cargo.toml`
- `crates/sql-lens-proxy/src/lib.rs`
- `Cargo.lock`
- `.trellis/spec/backend/quality-guidelines.md`

## Checklist

1. Add `sql-lens-config` path dependency to `sql-lens-proxy` if config conversion is implemented in proxy.
2. Import `BackendConfig` and `ProxyConfig` only for typed config conversion.
3. Define `BackendDialConfig`.
4. Implement `BackendDialConfig::new`.
5. Implement `BackendDialConfig::from_config`.
6. Define `ProxiedConnection`.
7. Add accessors or destructuring helpers for client stream, backend stream, client peer address, and backend address.
8. Define `BackendDialFailure`.
9. Define `BackendDialFailureKind`.
10. Define `BackendDialError`.
11. Implement `Display` and `std::error::Error` for `BackendDialError`.
12. Implement backend dialing with `tokio::time::timeout` around `TcpStream::connect`.
13. Preserve lightweight failure records on timeout and connect errors.
14. Add async test for config mapping.
15. Add async test for successful backend dial.
16. Add async test for refused/failed backend dial.
17. Add timeout test or document deterministic timeout limitation in task results.
18. Update backend spec with backend dialing contract.
19. Run validation.
20. Verify no forwarding, protocol parsing, capture, or app startup behavior was introduced.

## Validation Commands

```bash
rtk cargo fmt --check
rtk cargo check --workspace
rtk cargo test --workspace
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk cargo tree -p sql-lens-proxy
rtk rg -n "copy_bidirectional|AsyncReadExt|AsyncWriteExt|protocol|capture|sql-lens-app|main\\(" crates/sql-lens-proxy
rtk python3 ./.trellis/scripts/task.py validate .trellis/tasks/07-06-implement-backend-dialing
```

## Review Gate

Do not implement:

- Client/backend byte forwarding.
- Connection lifecycle persistence.
- Connection IDs.
- Protocol parsing.
- Capture pipeline.
- CLI runtime integration.
- Signal handling.
- TLS handling.
- Retry policy.
- Backend pooling.

## Commit Plan

When implementation and validation pass, create one work commit:

```text
feat: implement backend dialing
```

Do not archive the task until the work commit exists.
