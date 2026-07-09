# Issue 009: Implement environment variable overrides

## Goal

Implement local developer environment variable overrides for startup
configuration, while aligning the configuration contract with SQL Lens's
local-first product boundary.

SQL Lens is a developer-local debugging proxy. It does not implement
application-layer Auth, RBAC, or CSRF flows in the open-source core.

## Requirements

- Add environment variable overrides for the existing Issue 009 scope:
  - `SQL_LENS_PROXY_LISTEN`
  - `SQL_LENS_BACKEND_ADDRESS`
  - `SQL_LENS_LOGGING_LEVEL`
- Apply overrides after TOML parsing and before config validation/runtime
  startup.
- Keep overrides deterministic and testable without mutating process-global
  environment in unit tests.
- Reject invalid `SQL_LENS_LOGGING_LEVEL` values with a typed config override
  error.
- Keep multi-target behavior conservative: legacy `[proxy]` and `[backend]`
  env overrides do not rewrite explicit `[[targets]]` entries.
- Remove application-layer auth configuration and planning references from
  backend contracts. Preserve MySQL protocol authentication observation rules
  because those describe database wire-protocol state, not SQL Lens app auth.
- Leave replay execution, statistics WebSocket, Auth, RBAC, and CSRF out of
  scope.

## Acceptance Criteria

- [ ] `SqlLensConfig` can apply env overrides from the current process
      environment.
- [ ] Tests cover proxy listen, backend address, and logging level overrides.
- [ ] Invalid logging level override returns a structured error.
- [ ] App startup applies env overrides before validation.
- [ ] `AuthConfig`/`AuthMode` are removed from the startup configuration model.
- [ ] Project docs/specs no longer describe app-level Auth, RBAC, or CSRF as
      planned backend work.
- [ ] MySQL wire-protocol authentication safety rules remain documented.
- [ ] `rtk cargo fmt --check` passes.
- [ ] `rtk cargo test --workspace` passes.
- [ ] `rtk cargo clippy --workspace --all-targets -- -D warnings` passes.

## Notes

- Keep `prd.md` focused on requirements, constraints, and acceptance criteria.
- Lightweight tasks can remain PRD-only.
- For complex tasks, add `design.md` for technical design and `implement.md` for execution planning before `task.py start`.
