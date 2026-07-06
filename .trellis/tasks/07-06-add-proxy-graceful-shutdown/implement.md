# Proxy Graceful Shutdown Implementation Plan

## Preconditions

- Planning artifacts are reviewed.
- Task is activated with `task.py start`.
- Implementation remains limited to config contracts and proxy-local shutdown primitives.

## Files To Modify

- `crates/sql-lens-config/src/lib.rs`
- `crates/sql-lens-proxy/src/lib.rs`
- `CONFIG.md`
- `.trellis/spec/backend/quality-guidelines.md`
- `Cargo.lock` only if needed

## Checklist

1. Add `shutdown_timeout_ms` to `ProxyConfig`.
2. Set a conservative default, recommended `10_000`.
3. Update config tests for defaults and TOML parsing.
4. Update `CONFIG.md` proxy example and field description.
5. Define `ProxyShutdownConfig` in `sql-lens-proxy`.
6. Implement `ProxyShutdownConfig::new`.
7. Implement `ProxyShutdownConfig::from_config`.
8. Define `ProxyShutdownSignal`.
9. Implement subscribe/request shutdown helpers around `watch<bool>`.
10. Define `ProxyShutdownError` if request failure needs structured reporting.
11. Define `ShutdownDrainSummary`.
12. Define `ActiveSessionDrain`.
13. Implement bounded drain over active session handles.
14. Abort remaining handles after drain timeout.
15. Add tests for config default and TOML override.
16. Add tests for shutdown signal notification.
17. Add tests for successful drain.
18. Add tests for drain timeout.
19. Reuse existing listener shutdown tests; add coverage only if needed.
20. Update backend spec with graceful shutdown contract.
21. Run validation.
22. Verify no app startup, OS signal handling, protocol parsing, capture, storage, or lifecycle persistence was introduced.

## Validation Commands

```bash
rtk cargo fmt --check
rtk cargo check --workspace
rtk cargo test --workspace
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk cargo tree -p sql-lens-proxy
rtk rg -n "tokio::signal|ctrl_c|protocol|capture|sql-lens-app|main\\(|sql_lens_storage|sql_lens_api" crates/sql-lens-proxy crates/sql-lens-config
rtk python3 ./.trellis/scripts/task.py validate .trellis/tasks/07-06-add-proxy-graceful-shutdown
```

## Review Gate

Do not implement:

- OS signal handling.
- CLI/app runtime startup.
- Full session orchestration loop.
- Protocol parsing.
- SQL capture.
- Storage writes.
- Connection lifecycle persistence.
- Connection IDs.
- TLS handling.
- Retry policy.
- Backend pooling.

## Commit Plan

When implementation and validation pass, create one work commit:

```text
feat: add proxy graceful shutdown
```

Do not archive the task until the work commit exists.
