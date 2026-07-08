# Issue 064: Create React web app skeleton

## Goal

Create the initial React + TypeScript web application structure for the SQL Lens UI. The skeleton must match the directory layout in `UI.md`, build cleanly, and contain no hardcoded backend coupling. This unblocks parallel frontend development (Issues 066, 067, and feature work) while the backend API is built by another agent.

## Requirements

- Vite-based React + TypeScript project that builds cleanly (`npm run build` succeeds).
- Project located at `crates/sql-lens-app/web/` — co-located with the `sql-lens` binary that will eventually serve it, and outside the Cargo workspace so Rust builds ignore it.
- TailwindCSS configured.
- shadcn/ui initialized (components.json + utils present, one smoke component added).
- React Router with the six primary navigation routes from `UI.md`: Dashboard, SQL, Connections, Statistics, Replay, Settings.
- Application layout shell: left sidebar navigation, top bar, main content area.
- Light and dark theme with a working toggle that persists to localStorage; status color tokens (OK=green, Slow=amber, Error=red, Unknown=neutral).
- Directory layout matches the "Recommended React Structure" in `UI.md`.
- No backend coupling is hardcoded: no concrete `fetch` / XHR / WebSocket calls to API endpoints. The API base URL is read from a `VITE_API_BASE_URL` environment variable by a config module only; endpoint-coupled client functions are deferred to Issue 066.

## Out of Scope (deferred to follow-up issues)

- API client functions (Issue 066).
- TanStack Query providers and live data wiring (Issue 067).
- Monaco Editor and ECharts integration (feature issues for SQL Detail and Statistics).
- Full page implementations — this task ships route stubs and the shell only.

## Acceptance Criteria

- [ ] `npm run build` succeeds in `crates/sql-lens-app/web/`.
- [ ] `npm run dev` starts a dev server without errors.
- [ ] Directory layout under `web/src/` matches `UI.md` "Recommended React Structure".
- [ ] All six primary nav routes resolve and render stub content.
- [ ] Layout shell (sidebar + topbar + main) is present and responsive enough to not break.
- [ ] Light/dark theme toggle works and persists across reload.
- [ ] No concrete backend endpoint calls exist in the skeleton.
- [ ] `web/README.md` documents dev/build commands and the env var.

## Constraints

- Frontend is standalone; it must build without the Rust backend running.
- Do not modify Rust crates in this task.
- Node package manager: npm.
- Keep the dependency surface lean — defer Monaco, ECharts, and TanStack Query to their own issues.
