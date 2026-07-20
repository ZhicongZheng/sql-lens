# Default Static UI Delivery

## Goal

Make the local web dashboard reachable from the SQL Lens HTTP listener after a documented frontend build, using existing `web.static_dir` SPA serving.

## Background

- `HttpServerConfig.static_dir` + SPA fallback already work when configured.
- `WebConfig.static_dir` defaults to `None`.
- Docs examples use `crates/sql-lens-app/web/dist`.
- Frontend: `npm run build` → `dist/`.

## Requirements

1. Document standardized build + `static_dir` path for single-process local use.
2. Verify config wiring `SqlLensConfig.web` → HTTP server; fix if broken.
3. Optional low-cost helper (script/Makefile) if useful.
4. Valid `static_dir` serves UI without shadowing API/WS.
5. Unset = API-only; invalid explicit path = clear startup error.

## Acceptance Criteria

- [ ] README + CONFIG document build command, example/default path, single-process UI+API.
- [ ] With built dist + config, GET `/` serves SPA; health API still works.
- [ ] Invalid configured static dir fails startup clearly.
- [ ] Documented path does not require a separate Vite dev server.
- [ ] Touched validation passes.

## Out Of Scope

- Binary-embedded UI (rust-embed).
- Full M17 release packaging CI.
- Frontend feature work.
- UI auth.
