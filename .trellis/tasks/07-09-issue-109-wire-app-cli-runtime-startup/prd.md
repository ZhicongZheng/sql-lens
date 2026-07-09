# Issue 109: Wire app CLI runtime startup

## Goal

Wire sql-lens --config startup to the long-running backend runtime so local demo can start API and proxy services.

## Requirements

- `sql-lens --config sql-lens.toml` must load and validate the selected config, initialize logging, then start long-running backend runtime services.
- The HTTP API server must bind to `web.listen`.
- The proxy runtime must start one listener for each effective target:
  - explicit `[[targets]]` entries when present;
  - otherwise the legacy `[proxy]` + `[backend]` pair.
- Proxy capture, REST API, WebSocket broadcast, live statistics, and ring-buffer event storage must share one `ApiState`.
- Runtime startup must log the bound API address and each target listener address without logging credentials, SQL text, or packet payloads.
- Ctrl-C must trigger graceful shutdown for API and proxy listeners.
- Startup failures must return non-zero exit with clear human-readable errors.
- Default tests must not require a live database or Docker.

## Out of Scope

- Serving the React production bundle from the Rust app.
- Wiring SQLite persistence into app runtime.
- Replay execute, auth, TLS termination, config hot reload, or storage retention scheduling.
- Frontend changes.

## Acceptance Criteria

- [x] CLI starts API and proxy listeners from config and remains running until shutdown.
- [x] API server uses the configured `web.listen`.
- [x] Proxy listeners use all effective targets.
- [x] Runtime uses one shared `ApiState` for captured events, live statistics, and WebSocket broadcast.
- [x] Ctrl-C graceful shutdown is covered by tests or a deterministic shutdown primitive.
- [x] Ephemeral-port startup test does not require a live backend connection.
- [x] `rtk cargo fmt --check` passes.
- [x] `rtk cargo test -p sql-lens-app` passes.
- [x] `rtk cargo test --workspace` passes.
- [x] `rtk cargo clippy --workspace --all-targets -- -D warnings` passes.

## Notes

- Keep `prd.md` focused on requirements, constraints, and acceptance criteria.
- Lightweight tasks can remain PRD-only.
- For complex tasks, add `design.md` for technical design and `implement.md` for execution planning before `task.py start`.
