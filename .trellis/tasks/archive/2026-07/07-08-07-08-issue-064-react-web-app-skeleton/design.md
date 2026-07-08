# Design — Issue 064: React web app skeleton

## Location

`crates/sql-lens-app/web/`.

Rationale: the `sql-lens` binary (in `crates/sql-lens-app`) will eventually serve the built UI as static assets. Co-location keeps the serving path discoverable and keeps "the app" together. The directory is not a Cargo member and contains no `Cargo.toml`, so Rust builds ignore it. `UI.md` writes the structure as `web/src/...`; we honor that as the relative layout inside the crate directory.

## Stack (this task)

- **Vite + React 18 + TypeScript** — project scaffold.
- **TailwindCSS v4** via `@tailwindcss/vite` — CSS-first config, single `@import "tailwindcss"` entry.
- **shadcn/ui** — `components.json`, `src/lib/utils.ts` (`cn`), and a `Button` smoke component to prove the pipeline. Theme via CSS variables (shadcn defaults).
- **React Router v7** (`react-router-dom`) — declarative routes.

Deferred (own issues): TanStack Query (067), API client (066), Monaco + ECharts (feature issues).

## Directory Layout (matches UI.md "Recommended React Structure")

```text
crates/sql-lens-app/web/
  package.json
  vite.config.ts
  tsconfig.json
  tsconfig.node.json
  components.json          # shadcn
  index.html
  README.md
  .gitignore              # node_modules, dist
  public/
  src/
    main.tsx
    App.tsx
    app/
      routes/             # one stub per primary nav route
        dashboard.tsx
        sql-events.tsx
        connections.tsx
        statistics.tsx
        replay.tsx
        settings.tsx
      providers/
        theme-provider.tsx
    components/
      ui/                 # shadcn primitives (button)
      layout/
        app-shell.tsx     # sidebar + topbar + main
        sidebar.tsx
        topbar.tsx
      charts/             # placeholder (ECharts later)
      sql/                # placeholder (feature later)
      connections/        # placeholder (feature later)
    features/             # feature folders, stub index only
      dashboard/
      sql-events/
      connections/
      statistics/
      replay/
      settings/
    lib/
      api/
        config.ts         # reads VITE_API_BASE_URL; NO endpoint calls
      websocket/          # placeholder (feature later)
      format/             # placeholder
      filters/            # placeholder
      utils.ts            # cn() (shadcn)
    types/                # placeholder (typed API models come with 066)
    styles/
      globals.css         # tailwind import + theme tokens
```

## Routing

`react-router-dom` v7 with `createBrowserRouter` (or `<Routes>`). Routes:

| Path           | Component            |
| -------------- | -------------------- |
| `/`            | redirect → `/dashboard` |
| `/dashboard`   | Dashboard stub       |
| `/sql`         | SQL Events stub      |
| `/connections` | Connections stub     |
| `/statistics`  | Statistics stub      |
| `/replay`      | Replay stub          |
| `/settings`    | Settings stub        |
| `*`            | Not found            |

The sidebar renders nav links for the six primary routes; the active route is highlighted.

## Theme

- CSS variables for light/dark driven by a `.dark` class on `<html>` (shadcn convention).
- `ThemeProvider` exposes `theme` (`light` | `dark`) and `toggleTheme`; persists to `localStorage` key `sql-lens-theme`; applies/removes the `dark` class.
- No external dependency (no `next-themes`).
- Status color tokens documented in `styles/globals.css` as CSS variables: `--status-ok`, `--status-slow`, `--status-error`, `--status-unknown` mapped to green/amber/red/neutral.

## Backend Decoupling

- `src/lib/api/config.ts` exports `apiBaseUrl` derived from `import.meta.env.VITE_API_BASE_URL` with a default of `http://127.0.0.1:5173` (the API listener default from `ARCHITECTURE.md`). This is config only.
- No `fetch`, `XMLHttpRequest`, or `WebSocket` calls anywhere in the skeleton. Verified by grep in the check phase.
- Typed API models and client functions are explicitly Issue 066.

## Build & Verification

- `npm run build` = `tsc -b && vite build` (default Vite template build script). Must exit 0.
- `npm run dev` = Vite dev server.
- A `web/.gitignore` excludes `node_modules/` and `dist/`.
- Root `.gitignore` already ignores `/target/` and `.codegraph`; the web app adds its own.

## Risks / Tradeoffs

- **shadcn + Tailwind v4 in non-interactive shell**: shadcn CLI may prompt. Mitigation: use `--yes` / defaults; if the CLI is unreliable, hand-write `components.json` + `lib/utils.ts` + one component (the shadcn source is trivial and deterministic).
- **Tailwind v4 vs v3**: v4 is current and shadcn supports it; using v4 avoids a future migration. If v4 setup fails, fall back to v3 (`tailwindcss@3`, `postcss`, `autoprefixer`, `tailwind.config.js`) to unblock the build.
- **Lean deps**: intentionally not installing Monaco/ECharts/TanStack Query to keep the build fast and avoid coupling. Documented as deferred.

## Non-goals

- No Rust changes.
- No real data, no API wiring.
- No deployment / asset-embedding into the binary (later task).
