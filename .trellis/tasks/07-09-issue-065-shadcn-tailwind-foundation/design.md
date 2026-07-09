# Design — Issue 065: shadcn/ui and Tailwind foundation

## Location

All work inside `crates/sql-lens-app/web/`. No Rust crates touched. This runs
parallel to the backend multi-target-proxy task; the two share no files.

## Starting state (from Issue 064, verified)

- `vite.config.ts`: `react()` + `tailwindcss()` plugins, `@` alias → `./src`.
- `src/styles/globals.css`: `@import "tailwindcss"`, `@custom-variant dark`, full
  `:root` + `.dark` token blocks (shadcn defaults + `--status-*`), `@theme inline`
  mapping → `text-status-*` / `bg-status-*` utilities.
- `components.json`: style `new-york`, baseColor `neutral`, cssVariables true,
  iconLibrary `lucide`, aliases `@/components/ui` etc. **Already CLI-ready.**
- `src/lib/utils.ts`: `cn()` via clsx + tailwind-merge.
- `src/components/ui/button.tsx`: hand-written smoke component (cva-based).
- `src/app/providers/theme-provider.tsx`: `ThemeProvider` + `useTheme`, persists
  `sql-lens-theme`, toggles `.dark` on `document.documentElement` **in useEffect**
  (this is the FOUC source).
- `src/main.tsx`: `StrictMode > ThemeProvider > BrowserRouter > App`.
- `index.html`: bare head, no pre-hydration script.
- `node_modules/` and `package-lock.json` present; no Radix deps yet.

CLI support confirmed (shadcn docs via WebFetch): `npx shadcn@latest add <name>`
fully supports Tailwind v4 + Vite. 064 already satisfies all CLI prerequisites
(path aliases in tsconfig + vite, `@tailwindcss/vite` plugin).

## Component installation strategy

**Primary**: `npx shadcn@latest add <name> --yes` for each R1 component. The CLI
writes the file under `src/components/ui/<name>.tsx`, adds the Radix dependency
to `package.json`, and updates `package-lock.json`. Run from
`crates/sql-lens-app/web/`.

Batch the 17 components (R1 list) in one `add` call where the CLI supports it:
`npx shadcn@latest add table badge card dialog alert-dialog tabs tooltip
dropdown-menu select input skeleton scroll-area separator sheet sonner toggle
toggle-group --yes`. `button` already exists — the CLI skips/overwrites; pass
`--overwrite` only if needed, otherwise omit it to preserve 064's button.

**Fallback** (if CLI is interactive/unreliable in this shell, as 064's design
flagged): hand-write each `<name>.tsx` from the shadcn source (deterministic,
same approach 064 used for `button.tsx`) and `npm install` the declared Radix
deps (`@radix-ui/react-dialog`, `@radix-ui/react-tabs`, etc.) plus `sonner`,
`next-themes` is **not** used (064 rolled its own provider; keep it). The
fallback is labor-intensive but fully deterministic — use it only for components
the CLI fails on, not all.

## no-flash dark mode (R2)

Add an inline script to `index.html` `<head>`, before the module script:

```html
<script>
  (function () {
    try {
      var stored = localStorage.getItem("sql-lens-theme");
      var prefersDark =
        window.matchMedia("(prefers-color-scheme: dark)").matches;
      var dark = stored ? stored === "dark" : prefersDark;
      document.documentElement.classList.toggle("dark", dark);
    } catch (e) {}
  })();
</script>
```

- Storage key `sql-lens-theme` and class `dark` match `theme-provider.tsx`.
- The `try/catch` guards against `localStorage` being disabled (private mode).
- `theme-provider.tsx` is unchanged in behavior: its `useEffect` still
  re-applies/persists on toggle; the inline script only fixes first paint. The
  provider's `getInitialTheme` (already reads stored + prefers-color-scheme)
  stays in sync with the script, so no mismatch on hydration.
- Do **not** introduce `next-themes` — 064 deliberately avoided it; adding it
  now would duplicate the contract and require migrating the storage key.

## Global Toaster (R3)

shadcn `sonner` add creates `src/components/ui/sonner.tsx` exporting `<Toaster />`.
Mount once at the app root, inside `ThemeProvider` so toasts respect theme:

`src/main.tsx` — add `<Toaster richColors closeButton />` as a sibling of
`<BrowserRouter>` inside `<ThemeProvider>`:

```tsx
<ThemeProvider>
  <BrowserRouter>
    <App />
  </BrowserRouter>
  <Toaster richColors closeButton />
</ThemeProvider>
```

Any feature later calls `toast(...)` from `sonner` directly — no per-route wiring.

## Accessibility & focus baseline (R4)

- Rely on Radix defaults for focus trap, Esc-to-close, and `aria-*` in `dialog`,
  `alert-dialog`, `sheet`, `tabs`, `select`, `dropdown-menu`, `tooltip`. Do not
  strip these in wrappers.
- Icon-only controls: pass `aria-label` (review-enforced; no lint rule added in
  this task).
- `tooltip` paired with visible text or `aria-label` so it is not the sole
  information carrier.
- Status badges: `text-status-*` token + icon/word (re-assert R5).

## No hardcoded status colors (R5)

- `globals.css` already defines `--status-ok/slow/error/unknown` and surfaces
  them via `@theme inline` as `text-status-*` / `bg-status-*`.
- New code must not use `text-red-*` / `text-green-*` / `text-amber-*` /
  `text-yellow-*` / `text-emerald-*` / `text-rose-*` for **status**.
- `--destructive` (shadcn token) stays the sanctioned red for destructive
  **actions** (e.g. a "Delete" button), distinct from `--status-error`.
- Enforced by grep gate in acceptance criteria.

## Demo surface (R1 verification)

Reuse an existing route stub as a lightweight component showcase — not a
marketing page. Candidate: convert `src/app/routes/dashboard.tsx` (currently a
heading-only stub) into a `Card`-framed grid containing:

- a `Table` with `Badge` status cells (ok/slow/error/unknown),
- a `Tabs` block,
- a `Tooltip`-wrapped icon button,
- a `Dialog`/`AlertDialog` trigger,
- a `toast()`-firing button (sonner).

Keep it dense and tool-like (per `UI.md` Product Feel + component-guidelines
"no marketing pages"). This stays a stub — real Dashboard (069) replaces it
later.

If the user prefers isolation, an alternative is a temporary `/ui` route; default
is the dashboard stub reuse to avoid route sprawl.

## Dependency surface (post-task `package.json` additions)

Via CLI, expected Radix deps added: `@radix-ui/react-dialog`,
`@radix-ui/react-alert-dialog`, `@radix-ui/react-tabs`, `@radix-ui/react-tooltip`,
`@radix-ui/react-dropdown-menu`, `@radix-ui/react-select`,
`@radix-ui/react-scroll-area`, `@radix-ui/react-separator`,
`@radix-ui/react-sheet` (or `@radix-ui/react-dialog` for sheet in newer shadcn),
`@radix-ui/react-toggle`, `@radix-ui/react-toggle-group`, plus `sonner`.
`@radix-ui/react-slot` and `@radix-ui/react-label` may appear transitively.
**No** Monaco / ECharts / TanStack Query / `next-themes`.

## Build & verification

- `npm run build` (`tsc -b && vite build`) must exit 0 with all components.
- `npm run typecheck` (`tsc -b --noEmit`) must exit 0.
- Grep gates (R4/R5 + 064 decoupling) in implement.md validation section.
- `npm run dev` smoke: open the dashboard demo, toggle theme (no flash on
  reload), open dialog, fire a toast.

## Risks / Tradeoffs

- **CLI interactivity**: `npx shadcn@latest add` can prompt. Mitigation: `--yes`
  flag; fallback to hand-write (064 precedent) for any component that fails.
- **CLI overwriting 064's `button.tsx`**: omit `button` from the add list; if the
  CLI touches it, restore from git.
- **Radix version drift across components**: the CLI pins compatible versions,
  but a mixed install could conflict. Mitigation: install all in one CLI call so
  versions resolve together; run `npm install` + build after.
- **Sonner + React 18 StrictMode**: sonner is StrictMode-safe; no special
  handling needed.
- **Demo stub becoming a landing page**: risk called out in
  component-guidelines.md; mitigated by keeping it dense/table-driven.

## Rollback

- All deliverables live in `crates/sql-lens-app/web/`. `git checkout -- web/`
  reverts code; no Rust touched. `package-lock.json` reverts with it.
- Planning artifacts under `.trellis/tasks/07-09-issue-065-.../` are separate.

## Non-goals

- No Rust changes. No API client (066). No TanStack Query (067). No Monaco /
  ECharts. No full page implementations. No theme palette re-tuning.
