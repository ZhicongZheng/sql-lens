# Implement — Issue 065: shadcn/ui and Tailwind foundation

Execution checklist. Validate after each major step. All commands run from
`crates/sql-lens-app/web/` unless noted.

## 0. Pre-flight

- [ ] Confirm `node_modules/` present (`ls node_modules`); if missing, `npm install`.
- [ ] Confirm `npm run build` exits 0 on the 064 baseline before changing anything
      (establishes a clean before-state).
- [ ] Snapshot current `package.json` deps (`git show HEAD:crates/sql-lens-app/web/package.json`)
      so a CLI misfire is easy to diff.
- [ ] Confirm `button.tsx` is the only file under `src/components/ui/`.

## 1. Install shadcn base components (R1)

- [ ] Run the CLI in one batch (from `crates/sql-lens-app/web/`):
      `npx shadcn@latest add table badge card dialog alert-dialog tabs tooltip dropdown-menu select input skeleton scroll-area separator sheet sonner toggle toggle-group --yes`
- [ ] **Do not** add `button` (064's hand-written one is kept). If the CLI rewrote
      it, restore: `git checkout -- src/components/ui/button.tsx`.
- [ ] Verify each `<name>.tsx` exists under `src/components/ui/` and imports `cn`
      from `@/lib/utils`.
- [ ] Verify `package.json` gained the Radix deps + `sonner` and NO Monaco /
      ECharts / TanStack Query / `next-themes`.
- [ ] `npm install` (ensure lockfile consistent), then `npm run build` → exit 0.
- [ ] **Fallback** (only if CLI fails for a component): hand-write that
      `<name>.tsx` from the shadcn source and `npm install` its Radix dep
      explicitly. Note which components used the fallback in the task journal.

## 2. no-flash dark mode (R2)

- [ ] Edit `index.html`: add the inline pre-hydration script in `<head>` (see
      design.md R2) before `/src/main.tsx` module script.
- [ ] Leave `theme-provider.tsx` logic unchanged (provider + script agree on
      `sql-lens-theme` key + `dark` class).
- [ ] Smoke: set theme to dark, hard-reload → no light flash. Toggle to light,
      reload → stays light with `prefers-color-scheme: dark` fallback honored when
      no stored value (test by clearing localStorage).

## 3. Global Toaster (R3)

- [ ] Confirm `src/components/ui/sonner.tsx` exports `<Toaster />` (from step 1).
- [ ] Edit `src/main.tsx`: mount `<Toaster richColors closeButton />` inside
      `<ThemeProvider>`, as a sibling of `<BrowserRouter>` (see design.md R3).
- [ ] Smoke: call `toast("baseline ok")` from the demo surface (step 5) and
      confirm it renders and respects dark mode.

## 4. Accessibility & focus baseline (R4)

- [ ] Review each new component file: confirm Radix defaults (focus trap,
      Esc-to-close, `aria-*`) are **not** stripped.
- [ ] No code-level change expected here — it is a review/acceptance gate.
      Document any deliberate override in the journal.

## 5. Demo surface on dashboard stub (R1 verification)

- [ ] Replace `src/app/routes/dashboard.tsx` heading-only stub with a dense,
      tool-like showcase: `Card` frame containing a `Table` with `Badge` status
      cells (ok/slow/error/unknown using `text-status-*`), a `Tabs` block, a
      `Tooltip`-wrapped icon `Button`, a `Dialog` (or `AlertDialog`) trigger, and
      a `toast()`-firing button (sonner).
- [ ] Keep it stub-grade and dense — NOT a marketing landing page
      (component-guidelines.md).
- [ ] All sample data is local constants; no `fetch` / API coupling.
- [ ] `npm run build` → exit 0; `npm run dev` → dashboard renders all showcased
      components.

## 6. No hardcoded status colors (R5)

- [ ] Grep the new/changed files for ad-hoc status colors:
      `grep -rnE "text-(red|green|amber|yellow|emerald|rose)-[0-9]" src/`
      → no matches used for status. (`--destructive` token usage is allowed and
      noted.)
- [ ] Confirm status badges use `text-status-*` tokens + icon/word.

## Validation gates (run all before declaring done)

- [ ] `cd crates/sql-lens-app/web && npm run build` → exit 0.
- [ ] `cd crates/sql-lens-app/web && npm run typecheck` → exit 0.
- [ ] `grep -rnE "fetch\(|XMLHttpRequest|new WebSocket" crates/sql-lens-app/web/src/`
      → no matches (064 decoupling preserved).
- [ ] `grep -rn "/api/v1" crates/sql-lens-app/web/src/` → no runtime calls.
- [ ] `grep -rnE "text-(red|green|amber|yellow|emerald|rose)-[0-9]" crates/sql-lens-app/web/src/`
      → none used for status.
- [ ] All R1 components import via `@/components/ui/<name>` and use `cn`.
- [ ] `index.html` inline script present; dark reload has no flash.
- [ ] `<Toaster />` mounted once at app root.
- [ ] `package.json` has no Monaco / ECharts / TanStack Query / `next-themes`.
- [ ] Dashboard stub showcases: table + badge + card + dialog + tabs + tooltip +
      sonner.

## Spec update (Phase 3.3, after implementation)

- [ ] `.trellis/spec/frontend/component-guidelines.md`: add a "shadcn component
      inventory" subsection listing the installed primitives and the import
      convention (`@/components/ui/<name>`).
- [ ] `.trellis/spec/frontend/directory-structure.md`: note the `index.html`
      no-flash script contract and the `<Toaster />` mount location.
- [ ] `.trellis/spec/frontend/quality-guidelines.md`: add the
      `text-(red|green|amber|...)-[0-9]` → `text-status-*` grep gate alongside the
      existing decoupling grep gates.

## Rollback

- `git checkout -- crates/sql-lens-app/web/` reverts all code + lockfile changes.
  No Rust files touched. Planning artifacts under
  `.trellis/tasks/07-09-issue-065-.../` are separate.
