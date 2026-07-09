# Issue 112: Wire configured SQLite storage into app runtime

## Goal

Wire storage.type=sqlite into sql-lens-app runtime so captured events are persisted without blocking proxy forwarding.

## Requirements

- Runtime must keep the existing ring-buffer API state as the live API/UI view.
- When `storage.type = "sqlite"`, runtime must also initialize a SQLite event store using `storage.path`.
- SQLite initialization must fail startup with a clear runtime error if `storage.path` is empty, whitespace-only, or cannot be opened/migrated.
- Captured events must still update WebSocket broadcast, live statistics, and ring-buffer storage.
- Captured events must be sent to a SQLite persistence worker without blocking packet forwarding.
- SQLite persistence failures must be logged as warnings and must not stop proxy forwarding.
- Default `storage.type = "ring_buffer"` behavior must remain unchanged.
- This task must not change REST timeline/detail endpoints to read from SQLite.
- This task must not add frontend changes, plugin/exporter behavior, retention scheduling, OpenAPI, or auth.

## Acceptance Criteria

- [x] `storage.type = "ring_buffer"` starts runtime without SQLite.
- [x] `storage.type = "sqlite"` with `storage.path` starts runtime and creates/migrates the SQLite database.
- [x] `storage.type = "sqlite"` with an empty path returns a clear startup error.
- [x] Captured events are appended to ring buffer and live statistics as before.
- [x] Captured events are also persisted to SQLite through an async handoff.
- [x] SQLite worker insertion errors are logged and do not fail event capture.
- [x] Tests cover config-to-runtime SQLite initialization and persistence without Docker.
- [x] `rtk cargo fmt --check` passes.
- [x] `rtk cargo test -p sql-lens-app` passes.
- [x] `rtk cargo test -p sql-lens-storage` passes.
- [x] `rtk cargo test --workspace` passes.
- [x] `rtk cargo clippy --workspace --all-targets -- -D warnings` passes.

## Notes

- Keep `prd.md` focused on requirements, constraints, and acceptance criteria.
- Lightweight tasks can remain PRD-only.
- For complex tasks, add `design.md` for technical design and `implement.md` for execution planning before `task.py start`.
