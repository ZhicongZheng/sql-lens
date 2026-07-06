# Bidirectional TCP Forwarding Implementation Plan

## Preconditions

- Planning artifacts are reviewed.
- Task is activated with `task.py start`.
- Implementation remains limited to raw TCP forwarding in `sql-lens-proxy`.

## Files To Modify

- `crates/sql-lens-proxy/Cargo.toml`
- `crates/sql-lens-proxy/src/lib.rs`
- `Cargo.lock`
- `.trellis/spec/backend/quality-guidelines.md`

## Checklist

1. Enable Tokio `io-util` feature in `sql-lens-proxy`.
2. Import `tokio::io::copy_bidirectional`.
3. Define `TcpForwarder`.
4. Define `ForwardingSummary`.
5. Define `ForwardingFailure`.
6. Define `ForwardingError`.
7. Implement `Display` and `std::error::Error` for `ForwardingError`.
8. Implement `TcpForwarder::forward(ProxiedConnection)`.
9. Keep client stream as the first `copy_bidirectional` argument.
10. Map returned tuple to client-to-backend and backend-to-client counters.
11. Preserve client peer address and backend address in summary and errors.
12. Add async test helper to create a real `ProxiedConnection`.
13. Add test for client-to-backend copy.
14. Add test for backend-to-client copy.
15. Add test for bidirectional byte counters.
16. Add test for clean completion after one side closes.
17. Update backend spec with forwarding contract.
18. Run validation.
19. Verify no protocol parsing, capture, storage, app startup, or signal handling was introduced.

## Validation Commands

```bash
rtk cargo fmt --check
rtk cargo check --workspace
rtk cargo test --workspace
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk cargo tree -p sql-lens-proxy
rtk rg -n "protocol|capture|sql-lens-app|main\\(|sql_lens_storage|sql_lens_api" crates/sql-lens-proxy
rtk python3 ./.trellis/scripts/task.py validate .trellis/tasks/07-06-implement-bidirectional-tcp-forwarding
```

## Review Gate

Do not implement:

- Protocol parsing.
- SQL capture.
- Storage writes.
- Connection lifecycle persistence.
- Connection IDs.
- App runtime integration.
- Signal handling.
- Graceful shutdown orchestration.
- TLS handling.
- Retry policy.
- Backend pooling.

## Commit Plan

When implementation and validation pass, create one work commit:

```text
feat: implement bidirectional tcp forwarding
```

Do not archive the task until the work commit exists.
