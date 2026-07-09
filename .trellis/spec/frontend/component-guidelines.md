# Component Guidelines

> Component conventions for the planned SQL Lens frontend.

## Overview

The `web/` skeleton exists at `crates/sql-lens-app/web/` (Issue 064) with the app
shell (`src/components/layout/`), shadcn/ui primitives (`src/components/ui/`), and
route stubs (`src/app/routes/`) already in place. These rules define the React and
TypeScript UI contract; update this file with real component examples as feature
components are added.

SQL Lens is a developer inspection tool, not a marketing site. Components should
be dense, calm, and optimized for repeated debugging workflows.

## Component Structure

- Use React function components with TypeScript.
- Keep route-level feature composition under `web/src/features/*`.
- Keep reusable primitives and wrappers under `web/src/components/*`.
- Keep `components/ui` for shadcn/ui primitives and thin wrappers.
- Move feature-local components to shared folders only after a second feature
  needs them.
- Prefer small components with clear props over broad generic dashboards.

## Props Conventions

- Define explicit prop types or interfaces next to the component unless the type
  is shared across multiple files.
- Avoid `any`; API payloads should use shared frontend types aligned with backend
  response schemas.
- Keep command callbacks named by user intent, such as `onSelectEvent` or
  `onPauseStream`.
- Use discriminated unions for component modes when a component has mutually
  exclusive states.

## Styling Patterns

- Use TailwindCSS v4 and shadcn/ui as the base styling system. The `cn()` helper
  lives at `src/lib/utils.ts`; the path alias `@/*` resolves to `src/*`.
- Use the status color tokens (`text-status-ok` / `-slow` / `-error` /
  `-unknown`) defined in `src/styles/globals.css` — never hardcode `text-red-500`
  and similar for status. See directory-structure.md "Theme tokens".
- The `--destructive` token is for destructive **actions** (e.g. a "Delete"
  button), not for status. Status uses `--status-*`. Keep the two distinct.
- Keep tool surfaces compact and predictable: tables, split panes, filters,
  tabs, badges, and detail panels are preferred over decorative cards.
- Use status badges and restrained color to distinguish `ok`, `error`, and
  in-flight states. Color is never the only signal — pair with text or an icon.
- Do not render SQL text, parameters, or database errors as HTML.

## shadcn/ui Component Inventory (Issue 065)

Base primitives live under `src/components/ui/` and are imported via
`@/components/ui/<name>`. Installed set:

`button`, `table`, `badge`, `card`, `dialog`, `alert-dialog`, `tabs`,
`tooltip`, `dropdown-menu`, `select`, `input`, `skeleton`, `scroll-area`,
`separator`, `sheet`, `sonner` (Toaster), `toggle`, `toggle-group`.

Conventions:

- Add more with `npx shadcn@latest add <name> --yes` from
  `crates/sql-lens-app/web/`. If the CLI prompts to overwrite `button.tsx`,
  decline (it is the upgraded new-york-v4 version) unless intentionally
  re-pinning.
- Each primitive uses `cn()` from `@/lib/utils` and the `data-slot` hook pattern.
- The shadcn `sonner.tsx` was rewired to import `useTheme` from
  `@/app/providers/theme-provider` (not `next-themes`); do not reintroduce
  `next-themes` — 064 deliberately owns theme state.
- `TooltipProvider` is mounted once at the app root (`src/main.tsx`); individual
  `<Tooltip>` blocks do not need their own provider.
- Do not strip Radix defaults (focus trap, Esc-to-close, `aria-*`) when
  wrapping these primitives.

## Accessibility

- Buttons and icon-only controls need accessible labels.
- Tables should preserve readable headers and keyboard-friendly row actions.
- Dialogs and destructive replay actions must have explicit confirmation.
- Live updates should not steal focus while a user is inspecting data.

## Tests Required

For component changes:

- Component behavior tests for filters, selection, pause/resume, replay
  confirmation, and other logic-heavy UI.
- XSS tests or assertions for SQL text and database errors when rendering
  untrusted content.
- Playwright smoke tests for major flows once the app shell exists.
- Screenshots in PRs for visible UI changes.

## Layout Conventions (Issue 068)

- Desktop sidebar: collapsible (icon-only via `SidebarProvider`, persisted to
  `localStorage` key `sql-lens-sidebar-collapsed`). Nav items have lucide
  icons and tooltips when collapsed.
- Mobile sidebar: shadcn `Sheet` from the left, triggered by hamburger button
  in topbar (visible `< md` / `<768px`).
- Topbar: target badge (placeholder), capture status dot + label
  (`text-status-*`), search input, theme toggle (sun/moon icons).
- Right-side detail drawer: shadcn `Sheet` via `useDetailDrawer()` hook
  (`src/app/providers/detail-drawer-provider.tsx`). Opens programmatically,
  not by route. Content is a placeholder until SQL Detail / Connection Detail
  features land.
- Breakpoint handling: Tailwind `md:` prefix (≥768px). No JS `matchMedia`
  listener — CSS controls visibility.
- Provider nesting order in `main.tsx`: `ThemeProvider > SidebarProvider >
  DetailDrawerProvider > TooltipProvider > BrowserRouter`.

## Common Mistakes

- Do not build a landing page when the task asks for the SQL Lens app.
- Do not put cards inside cards or make dense operational screens look like
  marketing pages.
- Do not use `dangerouslySetInnerHTML` for SQL or database errors.
- Do not make live WebSocket updates reorder visible tables while the user is
  inspecting paused data.
