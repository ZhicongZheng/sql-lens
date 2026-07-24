# Implement multi-backend support

## Goal

Extend SQL Lens to support multiple backends simultaneously via sql-lens.toml: multi-backend config, per-backend dialers/listeners, backend-aware events/capture/replay, backward compatible with single-backend configs.

## Requirements

- Support multiple backends in `sql-lens.toml` via one `[[targets]]` entry per backend (backward compatible with single `[backend]`).
- Each target must have `name`, `listen`, `protocol`, `database_type`, and `backend_address`.
- Runtime: one listener + one dialer per target; events broadcast with `target_name`.
- Backend-aware capture, storage, and replay.
- Single-backend configs must behave identically.
- No breaking changes to CLI, API, or existing single-target behavior.

## Acceptance Criteria

- [x] Config loads multiple backends and single backend unchanged.
- [x] TOML validation passes for valid and invalid configs.
- [x] Runtime starts listeners and dialers for all configured backends.
- [x] Events and connection info include `backend_id` / `target_name`.
- [x] Replay and storage are backend-aware.
- [x] Existing tests and single-binary release still pass.

## Notes

- Keep `prd.md` focused on requirements, constraints, and acceptance criteria.
- Lightweight tasks can remain PRD-only.
- For complex tasks, add `design.md` for technical design and `implement.md` for execution planning before `task.py start`.
