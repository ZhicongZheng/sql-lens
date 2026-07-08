# Frontend Directory Structure

> Frontend code organization for SQL Lens.

## Overview

The SQL Lens frontend is a React and TypeScript developer tool UI. It should be dense, calm, and optimized for inspecting live SQL traffic.

The skeleton lives at `crates/sql-lens-app/web/`. It is **outside the Cargo workspace** (no `Cargo.toml`), so Rust builds ignore it. The `sql-lens` binary in `crates/sql-lens-app` will eventually serve the built `dist/` as static assets; that serving wiring is a later task and is out of scope for the skeleton.

## Directory Layout

Verified against `crates/sql-lens-app/web/` (Issue 064). Paths below are relative to `web/`:

```text
web/
├── package.json
├── vite.config.ts
├── tsconfig.json / tsconfig.app.json / tsconfig.node.json
├── components.json          # shadcn/ui config
├── index.html
├── README.md
├── .gitignore               # node_modules, dist
└── src/
    ├── main.tsx             # entry: BrowserRouter + ThemeProvider
    ├── App.tsx              # <Routes> with AppShell as layout
    ├── app/
    │   ├── routes/          # one stub per primary nav route
    │   └── providers/       # theme-provider.tsx
    ├── components/
    │   ├── ui/              # shadcn primitives (button.tsx)
    │   ├── layout/          # app-shell, sidebar, topbar, page-stub
    │   ├── charts/          # ECharts wrappers (placeholder)
    │   ├── sql/             # SQL display UI (placeholder)
    │   └── connections/     # connection UI (placeholder)
    ├── features/            # one folder per top-level view (placeholder barrels)
    │   ├── dashboard/
    │   ├── sql-events/
    │   ├── connections/
    │   ├── statistics/
    │   ├── replay/
    │   └── settings/
    ├── lib/
    │   ├── api/config.ts    # VITE_API_BASE_URL reader (config only)
    │   ├── websocket/       # placeholder
    │   ├── format/          # placeholder
    │   ├── filters/         # placeholder
    │   └── utils.ts         # cn() helper (shadcn)
    ├── types/               # API models land with Issue 066 (placeholder)
    └── styles/globals.css   # @import "tailwindcss" + theme tokens
```

## Module Organization

- `app`: routing, root providers, and app shell wiring.
- `components/ui`: shadcn/ui primitives and thin wrappers.
- `components/layout`: navigation, top bar, split panes, and page frames.
- `components/charts`: ECharts wrappers.
- `components/sql`: SQL display, parameter tables, status badges, and SQL-specific shared UI.
- `features/*`: route-level product features and feature-local components.
- `lib/api`: typed REST API client.
- `lib/websocket`: WebSocket client and subscription helpers.
- `lib/format`: duration, timestamp, SQL preview, and byte formatting.
- `types`: shared frontend types generated from or aligned with API schemas.

## State Rules

- TanStack Query owns server state.
- URL state owns durable filters.
- Component state owns temporary UI state.
- WebSocket events update query cache through explicit handlers.

## Naming Conventions

- React components use `PascalCase`.
- Hooks start with `use`.
- Feature folders use kebab-case.
- API JSON fields stay `snake_case`.
- Local TypeScript variables and properties use normal TypeScript conventions unless mirroring API payloads.

## Common Mistakes

- Do not render SQL text or database error messages as HTML.
- Do not use `any` for API payloads.
- Do not let live WebSocket updates make tables jump while the user is inspecting paused data.
- Do not put feature-specific components into global shared folders unless a second feature needs them.

## Scenario: API base URL wiring (frontend↔backend boundary)

### 1. Scope / Trigger
- Trigger: env wiring that couples the frontend to the backend API listener. Code-spec depth is mandatory because a wrong default or a hardcoded `fetch` breaks the decoupling contract that Issue 064 established.

### 2. Signatures
- `src/lib/api/config.ts` exports `const apiBaseUrl: string`.
- Reader: `import.meta.env.VITE_API_BASE_URL ?? "http://127.0.0.1:5173"`, trailing slash stripped.

### 3. Contracts
- Environment key `VITE_API_BASE_URL` (optional). Default `http://127.0.0.1:5173` matches the API listener recommended default in `ARCHITECTURE.md`.
- `apiBaseUrl` is the **base only** (origin + optional port). It is NOT a full endpoint and must not include a trailing slash or a `/api/v1` path segment.
- Runtime endpoint construction (base + `/api/v1/...`) is the job of the typed API client in Issue 066, not the skeleton's config module.

### 4. Validation & Error Matrix
- `VITE_API_BASE_URL` unset → use `127.0.0.1:5173` default (no error).
- `VITE_API_BASE_URL` set with a trailing slash → stripped at read time (no error).
- A `fetch(`/api/v1/...`)` or `new WebSocket(...)` call anywhere in the skeleton → **forbidden** (decoupling violation). The skeleton must build and render with no backend running.

### 5. Good/Base/Bad Cases
- Good: `src/lib/api/config.ts` reads the env var, exports `apiBaseUrl`, makes zero network calls.
- Base: a route stub imports nothing from `lib/api` and renders static text.
- Bad: a component calls `fetch(\`${apiBaseUrl}/api/v1/sql-events\`)` directly — that belongs in the Issue 066 client, not in a component or the config module.

### 6. Tests Required
- Grep assertion (run before declaring a skeleton/decoupling task done):
  `grep -rnE "fetch\(|XMLHttpRequest|new WebSocket" crates/sql-lens-app/web/src/` → no matches.
  `grep -rn "/api/v1" crates/sql-lens-app/web/src/` → no matches (comments excluded).
- Build assertion: `cd crates/sql-lens-app/web && npm run build` exits 0.

### 7. Wrong vs Correct
#### Wrong
```ts
// a route stub or component hardcodes a call
const res = await fetch("http://127.0.0.1:5173/api/v1/sql-events");
```
#### Correct
```ts
// src/lib/api/config.ts — config only, no calls
export const apiBaseUrl: string = (
  import.meta.env.VITE_API_BASE_URL ?? "http://127.0.0.1:5173"
).replace(/\/$/, "");
// endpoint usage deferred to the Issue 066 typed client
```

## Scenario: Theme tokens (light/dark + status colors)

### 1. Scope / Trigger
- Trigger: a cross-cutting display contract (status colors) referenced by many future components. Code-spec depth prevents each feature from inventing its own status palette.

### 2. Signatures
- `src/app/providers/theme-provider.tsx` exports `ThemeProvider`, `useTheme`.
- `useTheme()` returns `{ theme, toggleTheme, setTheme }` where `theme: "light" | "dark"`.

### 3. Contracts
- Persistence: `localStorage` key `sql-lens-theme`, value `"light"` | `"dark"`.
- Applied by toggling the `dark` class on `document.documentElement`.
- Initial value: stored value, else `prefers-color-scheme: dark` media query, else `"light"`.
- CSS variables defined in `src/styles/globals.css` for both `:root` and `.dark`:
  `--status-ok` (green), `--status-slow` (amber), `--status-error` (red), `--status-unknown` (neutral).
- Surfaced to Tailwind via `@theme inline` as `--color-status-ok/slow/error/unknown` → utility classes `text-status-*` / `bg-status-*`.

### 4. Validation & Error Matrix
- `useTheme` called outside `ThemeProvider` → throws `useTheme must be used within a ThemeProvider`.
- Theme toggle without prior `localStorage` → writes the new value and persists.
- Reload → persisted theme is reapplied before first paint (provider reads it in initializer + `useEffect`).

### 5. Good/Base/Bad Cases
- Good: a status badge uses `text-status-error` and an icon/word — color is not the only signal.
- Base: the topbar idle indicator uses `text-status-unknown`.
- Bad: a component hardcodes `text-red-500` for an error status instead of `text-status-error`.

### 6. Tests Required
- Build assertion: `npm run build` exits 0 (verifies the `@theme inline` mapping compiles and all `*-status-*` utilities resolve).
- (Future) Component test: toggling theme flips the `dark` class on `<html>` and survives a remount.

### 7. Wrong vs Correct
#### Wrong
```tsx
<span className="text-red-500">Error</span> // ad-hoc color, not a token
```
#### Correct
```tsx
<span className="text-status-error">Error</span> // status token + word
```

## Toolchain & Build (established by Issue 064)

- Package manager: **npm**. Scripts: `npm run dev` (Vite dev server, port 5174), `npm run build` (`tsc -b && vite build`), `npm run preview`, `npm run typecheck`.
- Stack: Vite 6 + React 18 + TypeScript (strict, `noUnusedLocals`/`noUnusedParameters` on) + TailwindCSS v4 (`@tailwindcss/vite`, CSS-first config via `@import "tailwindcss"` + `@theme inline`) + shadcn/ui (`components.json`, `lib/utils.ts` `cn`, New York style, neutral base) + React Router v7.
- Path alias: `@/* -> ./src/*`, configured in BOTH `tsconfig.app.json` (`paths`) and `vite.config.ts` (`resolve.alias`). Imports use `@/...` exclusively.
- The skeleton intentionally does NOT install Monaco Editor, ECharts, or TanStack Query — those land in their own issues (066, 067, feature work). Do not add them speculatively.


