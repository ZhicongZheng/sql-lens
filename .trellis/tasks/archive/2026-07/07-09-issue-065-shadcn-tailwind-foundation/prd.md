# Issue 065: Add shadcn/ui and Tailwind foundation

## Goal

Turn the React skeleton (from Issue 064) into a usable design-system baseline by
installing the shadcn/ui base components that downstream UI.md pages need, fixing
dark-mode flash-on-reload (FOUC), mounting a global Toaster, and codifying the
"no hardcoded status colors" + accessibility/focus baseline. This unblocks Issues
068 (app layout shell), 069 (Dashboard), and the SQL List / SQL Detail pages, so
they can compose real shadcn primitives instead of ad-hoc Tailwind.

## Background

Issue 064 shipped the skeleton at `crates/sql-lens-app/web/` with Tailwind v4 +
shadcn/ui already bootstrapped:

- `@tailwindcss/vite` plugin in `vite.config.ts`; `globals.css` with
  `@import "tailwindcss"` + full `@theme inline` mapping.
- `components.json` (new-york style, neutral base, cssVariables, lucide icons).
- `src/lib/utils.ts` exports `cn()` (clsx + tailwind-merge).
- `src/components/ui/button.tsx` as the only shadcn primitive (smoke component).
- `src/app/providers/theme-provider.tsx` with light/dark toggle persisting to
  `localStorage` key `sql-lens-theme`; status tokens `--status-ok/slow/error/unknown`
  in `globals.css` and surfaced via `@theme inline` as `text-status-*`.

This already satisfies the literal acceptance criteria in `ISSUES.md` (Issue 065:
Tailwind config exists / shadcn components importable / light+dark tokens exist).
Per user decision, Issue 065 is therefore scoped as a **design-system baseline**
expansion on top of 064 — not a re-do — adding the component inventory, no-flash
theme, Toaster, and codified constraints that 064 intentionally deferred.

Out of scope (deferred, per spec): Monaco Editor, ECharts, TanStack Query
(Issues 066/067 and feature work). No speculative installs beyond the page-driven
component list below.

## Requirements

### R1 — shadcn base component inventory

Install the following shadcn/ui primitives under `src/components/ui/` so they can
be imported via `@/components/ui/<name>`. `button.tsx` already exists and is not
re-added.

`table`, `badge`, `card`, `dialog`, `alert-dialog`, `tabs`, `tooltip`,
`dropdown-menu`, `select`, `input`, `skeleton`, `scroll-area`, `separator`,
`sheet`, `sonner`, `toggle`, `toggle-group`.

Each must (a) build under `npm run build`, (b) import `cn` from `@/lib/utils`,
(c) carry the Radix dependency the CLI declares (added to `package.json`).

Selection rationale (maps each component to a UI.md page need):

- `table` — SQL List columns, Connections columns, SQL Detail parameter table.
- `badge` — Status distinction (OK/Slow/Error/Unknown); never color-only.
- `card` — Dashboard widgets, page-level containers.
- `dialog` / `alert-dialog` — Replay mutation confirmation (UI.md Replay).
- `tabs` — SQL Detail sections (Summary / Original SQL / Expanded SQL / Parameters
  / Timings / Result / Error / Connection / Protocol / Replay).
- `tooltip` — Icon-only controls need accessible labels (UI.md Accessibility).
- `dropdown-menu` — Per-row actions, secondary menus.
- `select` — Protocol/Status/Database/User filters (SQL List controls).
- `input` — Text search, duration range (SQL List controls).
- `skeleton` — Loading states (empty/loading/error matrix).
- `scroll-area` — Long SQL lists, live event streams without page jump.
- `separator` — Section dividers in dense tool surfaces.
- `sheet` — Mobile filter drawer, mobile detail drawer (UI.md Mobile layout).
- `sonner` — Toast feedback for replay results, copy-to-clipboard, errors.
- `toggle` / `toggle-group` — Toggle original/expanded SQL (SQL Detail).

### R2 — Dark mode without flash on reload (FOUC fix)

Current `theme-provider.tsx` sets the `.dark` class in `useEffect`, so a reload in
dark mode flashes light-first before hydration. Fix by adding an inline script in
`index.html` `<head>` that runs before hydration:

- Read `localStorage` key `sql-lens-theme`; if absent, fall back to
  `prefers-color-scheme: dark`.
- Add/remove the `dark` class on `document.documentElement` accordingly.

`theme-provider.tsx` keeps owning the React-side state and persistence; the inline
script only guarantees first-paint correctness. The two must agree on the storage
key and class name (both already `sql-lens-theme` + `.dark`).

### R3 — Global Toaster mounted

Mount the shadcn/sonner `<Toaster />` once at the app root (in `main.tsx` or
`App.tsx`, inside `ThemeProvider`) so any feature can call `toast()` without
per-route wiring. Configure richColors and an accessible close button.

### R4 — Accessibility & focus baseline

- Icon-only buttons/controls have `aria-label` (enforced by review, not a lint
  rule).
- Dialogs (`dialog`, `alert-dialog`) and `sheet` rely on Radix focus trap and
  Esc-to-close defaults; do not override them in wrappers.
- `tooltip` is never the only carrier of information for an action — pair with an
  accessible label or visible text.
- Color is never the only status signal: status badges use `text-status-*` tokens
  **plus** an icon or word (already a spec rule; re-asserted here as a 065 gate).

### R5 — No hardcoded status colors (constraint, enforced)

No new code introduces `text-red-*` / `text-green-*` / `text-amber-*` / `text-yellow-*`
for status semantics. Use `text-status-ok` / `text-status-slow` /
`text-status-error` / `text-status-unknown` from `globals.css`. shadcn's
`--destructive` token remains the only sanctioned red (for destructive actions,
not status). Verified by grep before completion.

## Acceptance Criteria

- [ ] All R1 components exist under `src/components/ui/` and `npm run build` exits 0.
- [ ] Each R1 component imports via `@/components/ui/<name>` and uses `cn` from
      `@/lib/utils`.
- [ ] `index.html` has an inline pre-hydration script that sets `.dark` from
      `localStorage`/`prefers-color-scheme`; a dark-mode reload shows no light flash.
- [ ] `<Toaster />` (sonner) is mounted once at the app root inside `ThemeProvider`;
      a sample `toast()` call renders.
- [ ] `grep -rnE "text-(red|green|amber|yellow|emerald|rose)-[0-9]" src/` returns no
      matches used for status (destructive-token usage allowed, documented).
- [ ] `grep -rnE "fetch\(|XMLHttpRequest|new WebSocket" src/` → no matches (064
      decoupling contract preserved).
- [ ] `grep -rn "/api/v1" src/` → no runtime calls (config-only reader unchanged).
- [ ] No Monaco / ECharts / TanStack Query added to `package.json`.
- [ ] `npm run typecheck` (`tsc -b --noEmit`) exits 0.
- [ ] At least one route stub or a dedicated demo surface exercises a sampling of the
      new components (table + badge + card + dialog + tabs + tooltip + sonner) so the
      baseline is visually verifiable, without becoming a marketing landing page.

## Constraints

- Frontend standalone: builds with no Rust backend running.
- No Rust crate changes.
- Package manager: npm.
- Do not install Monaco, ECharts, or TanStack Query (their own issues).
- Parallel with the in-progress backend task (multi-target proxy); no shared files
  touched.

## Out of Scope

- API client functions (Issue 066).
- TanStack Query providers (Issue 067).
- Monaco Editor + ECharts integration (feature issues).
- Full page implementations (068 layout shell, 069 Dashboard, SQL List/Detail) —
  065 only provides the component inventory and design-system baseline.
- Theme color palette re-tuning beyond FOUC fix (064's oklch tokens are kept).
