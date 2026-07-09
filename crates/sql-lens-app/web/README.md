# SQL Lens Web UI

React + TypeScript skeleton for the SQL Lens web UI (Issue 064), with the
shadcn/ui + Tailwind design-system baseline (Issue 065).

## Stack

- Vite + React 18 + TypeScript
- TailwindCSS v4
- shadcn/ui (component primitives, new-york style, neutral base)
- React Router v7

Deferred to follow-up issues: TanStack Query (067), API client (066),
Monaco Editor and ECharts (feature work).

## Commands

```bash
npm install        # install dependencies
npm run dev        # start dev server (http://127.0.0.1:5174)
npm run build      # type-check + production build -> dist/
npm run preview    # preview the production build
npm run typecheck  # type-check only
```

## Configuration

The API base URL is read from the `VITE_API_BASE_URL` environment variable,
defaulting to `http://127.0.0.1:5173` (the Rust backend's recommended API
listener port). Override it for local development:

```bash
VITE_API_BASE_URL=http://127.0.0.1:5173 npm run dev
```

The skeleton intentionally contains **no backend coupling**: no concrete
`fetch`, `XMLHttpRequest`, or `WebSocket` calls. Typed API client functions
and live data wiring land in Issues 066 and 067.

## shadcn/ui components

Base primitives live under `src/components/ui/` and are imported via
`@/components/ui/<name>`. Issue 065 installed:

`button`, `table`, `badge`, `card`, `dialog`, `alert-dialog`, `tabs`,
`tooltip`, `dropdown-menu`, `select`, `input`, `skeleton`, `scroll-area`,
`separator`, `sheet`, `sonner` (Toaster), `toggle`, `toggle-group`.

Add more with the shadcn CLI (run from `crates/sql-lens-app/web/`):

```bash
npx shadcn@latest add <component> --yes
```

Do not add Monaco, ECharts, or TanStack Query here — they belong to their own
issues.

## Layout

See `src/` — the directory layout matches the "Recommended React Structure"
in the project root `UI.md`:

```text
src/
  app/        routes + providers
  components/ ui (shadcn), layout, charts, sql, connections
  features/   one folder per top-level view
  lib/        api (config only), websocket, format, filters, utils
  types/      API model types (Issue 066)
  styles/     globals.css (tailwind + theme tokens)
```

## Theme

Light/dark toggle persists to `localStorage` under `sql-lens-theme`. An inline
script in `index.html` applies the theme **before hydration** so a reload in
dark mode does not flash light-first. `src/app/providers/theme-provider.tsx`
owns the React-side state and persistence; the two agree on the storage key
and the `.dark` class on `<html>`.

Status color tokens are defined in `src/styles/globals.css`:

| Status   | Variable           | Color  |
| -------- | ------------------ | ------ |
| OK       | `--status-ok`      | green  |
| Slow     | `--status-slow`    | amber  |
| Error    | `--status-error`   | red    |
| Unknown  | `--status-unknown` | neutral |

Color is never the only status signal — pair with text or an icon. Use the
`text-status-*` / `bg-status-*` utilities; never hardcode `text-red-*` etc.
for status (the `--destructive` token is for destructive **actions**, not
status).

## Toasts

`<Toaster richColors closeButton />` (sonner) is mounted once at the app root
in `src/main.tsx`, inside `ThemeProvider`. Any feature can call `toast(...)`
from `sonner` directly — no per-route wiring.
