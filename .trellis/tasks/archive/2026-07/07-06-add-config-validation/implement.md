# Config Validation Implementation Plan

## Preconditions

- Planning artifacts are reviewed.
- Task is activated with `task.py start`.
- Implementation remains limited to `sql-lens-config` validation and tests.

## Files To Modify

- `crates/sql-lens-config/src/lib.rs`

Optional spec update if implementation establishes reusable validation conventions:

- `.trellis/spec/backend/quality-guidelines.md`

## Checklist

1. Add `ConfigValidationViolation`.
2. Add `ConfigValidationError`.
3. Implement `Display` and `std::error::Error` for `ConfigValidationError`.
4. Add `SqlLensConfig::validate`.
5. Collect `MissingProxyListen` for empty or whitespace-only `proxy.listen`.
6. Collect `MissingBackendAddress` for empty or whitespace-only `backend.address`.
7. Collect `UnsupportedProtocol` for non-MySQL `proxy.protocol`.
8. Add tests for valid default config.
9. Add tests for missing and whitespace proxy listen.
10. Add tests for missing and whitespace backend address.
11. Add tests for unsupported protocol.
12. Add tests for multiple violations returned together.
13. Add trait/display tests for validation error.
14. Run validation commands.
15. Verify no out-of-scope behavior was introduced.

## Validation Commands

```bash
rtk cargo fmt --check
rtk cargo check --workspace
rtk cargo test --workspace
rtk cargo clippy --workspace --all-targets -- -D warnings
rtk rg -n "notify|clap|tokio|axum|rusqlite|sql-lens-core" crates/sql-lens-config
rtk python3 ./.trellis/scripts/task.py validate .trellis/tasks/07-06-add-config-validation
```

## Review Gate

Do not implement:

- Environment overrides.
- CLI `--config`.
- Runtime startup.
- Hot reload.
- File watching.
- Address parsing.
- Storage capacity validation.
- TLS certificate validation.
- Auth validation.
- Protocol registry integration.

## Commit Plan

When implementation and validation pass, create one work commit:

```text
feat: add config validation
```

Do not archive the task until the work commit exists.
