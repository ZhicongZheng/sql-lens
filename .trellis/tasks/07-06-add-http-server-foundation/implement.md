# Add HTTP Server Foundation Implementation Plan

## Checklist

- [x] Start the task with `rtk python3 ./.trellis/scripts/task.py start .trellis/tasks/07-06-add-http-server-foundation`.
- [x] Load `trellis-before-dev` before editing implementation files.
- [x] Add focused dependencies to `crates/sql-lens-api/Cargo.toml`.
- [x] Split `sql-lens-api` into small modules:
  - `server` for config, bind, serve, and server errors.
  - `request_id` for request ID header/middleware behavior.
  - `tests` or module-local tests for the foundation behavior.
- [x] Re-export the public API from `src/lib.rs`.
- [x] Implement `HttpServerConfig` conversion from `sql_lens_config::WebConfig`.
- [x] Implement `bind_http_server` using `tokio::net::TcpListener`.
- [x] Implement graceful shutdown with `axum::serve(listener, router).with_graceful_shutdown(shutdown)`.
- [x] Implement request ID middleware with `x-request-id` propagation.
- [x] Add lightweight async tests for bind, shutdown, generated request ID, and incoming request ID propagation.
- [x] Confirm `sql-lens-app` tests still pass without changing app runtime behavior.
- [x] Run the validation commands.
- [x] Update planning or specs only if implementation reveals a contract mismatch.

## Validation Commands

Run narrow validation first:

```bash
rtk cargo test -p sql-lens-api
```

Then run full backend validation:

```bash
rtk cargo fmt --check
rtk cargo check --workspace
rtk cargo test --workspace
rtk cargo clippy --workspace --all-targets -- -D warnings
```

## Risk Points

- Axum and Tower type inference can make middleware return types verbose. Prefer simple functions and explicit public wrappers over exposing complicated generic types.
- Request ID generation must be deterministic enough for tests without introducing global mutable state that makes tests flaky.
- Avoid accidentally adding a health endpoint, app runtime wiring, or signal handling; those are separate tasks.
- Avoid fixed test ports.

## Rollback Points

- If framework dependency choices become too large, revert `sql-lens-api/Cargo.toml` and module files before touching other crates.
- If request ID middleware becomes too framework-specific, keep the public header constant and server bind/shutdown code, then revisit middleware design before starting implementation.
