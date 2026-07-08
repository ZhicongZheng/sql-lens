# Implement — Issue 064: React web app skeleton

Execution checklist. Validate after each major step.

## 1. Scaffold Vite project

- [ ] From `crates/sql-lens-app/`, scaffold into `web/`: `npm create vite@latest web -- --template react-ts`
- [ ] `cd web && npm install`
- [ ] Verify: `npm run build` exits 0 (vanilla template).

## 2. TailwindCSS v4

- [ ] `npm install tailwindcss @tailwindcss/vite`
- [ ] Add `tailwindcss()` plugin to `vite.config.ts`.
- [ ] Replace `src/index.css` content with `@import "tailwindcss";` (rename/move to `src/styles/globals.css` and import from `main.tsx`).

## 3. shadcn/ui

- [ ] Ensure `tsconfig.json` has path alias `@/* -> ./src/*` and `baseUrl`.
- [ ] `npm install -D @types/node` (for path alias in vite config).
- [ ] Set `vite.config.ts` `resolve.alias` `@ -> /src`.
- [ ] `npx shadcn@latest init` (use `--yes`, base color `neutral`). If interactive/unreliable, hand-create `components.json`, `src/lib/utils.ts` (`cn` via `clsx` + `tailwind-merge`), and the theme CSS variables.
- [ ] `npx shadcn@latest add button --yes`
- [ ] Verify: build still exits 0; `Button` imports cleanly.

## 4. Theme provider

- [ ] Write `src/app/providers/theme-provider.tsx`: context with `theme`, `toggleTheme`, localStorage persistence (`sql-lens-theme`), toggles `.dark` on `document.documentElement`.
- [ ] Add status color CSS variables to `globals.css` (`--status-ok/slow/error/unknown`).
- [ ] Wrap `<App>` with `ThemeProvider` in `main.tsx`.

## 5. Layout shell

- [ ] `src/components/layout/sidebar.tsx` — nav links for the six routes, active highlight (NavLink).
- [ ] `src/components/layout/topbar.tsx` — theme toggle + placeholder for capture status / search.
- [ ] `src/components/layout/app-shell.tsx` — sidebar + topbar + `<Outlet />`.
- [ ] Use the shadcn `Button` for the theme toggle.

## 6. Routing + route stubs

- [ ] `npm install react-router-dom`
- [ ] Create `src/app/routes/{dashboard,sql-events,connections,statistics,replay,settings}.tsx` — each a minimal stub page with a heading.
- [ ] `src/App.tsx` — `<Routes>` with `app-shell` as the element wrapping nested routes; `/` redirects to `/dashboard`; `*` → not found.

## 7. Directory layout completion

- [ ] Create placeholder dirs/files to match UI.md: `components/{charts,sql,connections}`, `features/{dashboard,sql-events,connections,statistics,replay,settings}` (each with an `index.ts` barrel or `.gitkeep`), `lib/{websocket,format,filters}`, `types/`.
- [ ] `src/lib/api/config.ts` — reads `import.meta.env.VITE_API_BASE_URL`, default `http://127.0.0.1:5173`, exports `apiBaseUrl`. No fetch.

## 8. Docs + gitignore

- [ ] `web/README.md` — dev/build commands, env var, layout overview.
- [ ] `web/.gitignore` — `node_modules`, `dist`, `*.local`.

## Validation gates

- [ ] `cd crates/sql-lens-app/web && npm run build` → exit 0.
- [ ] `npm run dev` starts without error (smoke; Ctrl-C).
- [ ] `grep -rnE "fetch\(|XMLHttpRequest|new WebSocket" src/` → no matches (no backend coupling).
- [ ] `grep -rn "/api/v1" src/` → no runtime calls (only possibly a comment in config.ts).
- [ ] Directory tree under `src/` matches design.md / UI.md.
- [ ] All six routes render.

## Rollback

- The entire deliverable lives in `crates/sql-lens-app/web/` (plus planning artifacts in `.trellis/tasks/...`). `rm -rf crates/sql-lens-app/web` reverts the code; no Rust files are touched.
