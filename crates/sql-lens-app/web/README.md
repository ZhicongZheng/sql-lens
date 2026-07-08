# SQL Lens Web UI

React + TypeScript skeleton for the SQL Lens web UI (Issue 064).

## Stack

- Vite + React 18 + TypeScript
- TailwindCSS v4
- shadcn/ui (component primitives)
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

Light/dark toggle persists to `localStorage` under `sql-lens-theme`. Status
color tokens are defined in `src/styles/globals.css`:

| Status   | Variable           | Color  |
| -------- | ------------------ | ------ |
| OK       | `--status-ok`      | green  |
| Slow     | `--status-slow`    | amber  |
| Error    | `--status-error`   | red    |
| Unknown  | `--status-unknown` | neutral |

Color is never the only status signal — pair with text or an icon.
