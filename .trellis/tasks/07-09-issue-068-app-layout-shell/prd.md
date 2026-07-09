# Issue 068: Build app layout shell

## Goal

Transform the layout shell stubs into a responsive, production-ready layout
that supports desktop and mobile use. This unblocks all downstream feature
pages (069 Dashboard, SQL List, SQL Detail, Connections, etc.) by providing a
stable navigation and content frame.

## Requirements

### R1 — Sidebar: icons + collapse + mobile drawer

- Each of the 6 nav items gets a lucide icon (Dashboard=`LayoutDashboard`,
  SQL=`Database`, Connections=`Link`, Statistics=`BarChart3`, Replay=`Play`,
  Settings=`Settings`).
- Sidebar is collapsible to icon-only mode (64px wide) via a toggle button.
  Expanded width stays 224px. Collapse state persists to
  `localStorage` (`sql-lens-sidebar-collapsed`).
- On `< md` breakpoint (`<768px`): sidebar is hidden by default; a hamburger
  button in the topbar opens it as a shadcn `Sheet` from the left. The Sheet
  contains the same nav items with icons and labels.
- Sidebar uses `aria-label="Primary"` on the nav element (already present).
  The collapse toggle has `aria-label` (e.g. "Toggle sidebar").

### R2 — Topbar: target indicator + capture status + search

- **Active target indicator**: displays the currently selected proxy target
  name (e.g. `mysql-local`). This is a placeholder display for now — the
  actual target data comes from the API (066) and backend config. Use a
  hardcoded default (`mysql-local`) with a Badge; wiring to real data is a
  follow-up.
- **Capture status**: a status indicator showing capture state
  (`active`/`paused`/`stopped`). Visual: a small dot + label using
  `text-status-*` tokens. Default: `active` (green dot). State toggle is
  placeholder for now; wiring to WebSocket (067) is a follow-up.
- **Global search**: a text Input with search icon (lucide `Search`) in the
  topbar. This is a visual placeholder — the search handler wiring to
  SQL List filters is a follow-up. The input has `placeholder="Search
  SQL..."` and `aria-label="Search SQL"`.
- **Theme toggle**: keep the existing `Sun`/`Moon` icon toggle (already in
  topbar.tsx). Move it to the right side of the topbar.
- **Sidebar toggle** (mobile): a hamburger button (`Menu` icon) visible only
  on `< md`, opens the Sheet sidebar.
- Topbar height stays `h-12`. Content is horizontally spaced with flex.

### R3 — Right-side detail drawer framework

- A `Sheet` component on the right side, opened programmatically (not by
  route). This is a framework placeholder — no content yet.
- Exposed via a context/hook: `useDetailDrawer()` returns `{ isOpen,
  openDrawer, closeDrawer }`. The Sheet renders an empty placeholder
  "Detail panel — content arrives with SQL Detail (Issue 0xx)".
- Width: `sm:max-w-lg` (shadcn Sheet default right side).
- The drawer does NOT overlap the main content on desktop; the main area
  compresses when the drawer is open (use flex, not overlay).

### R4 — Responsive breakpoints

- `>= md` (≥768px): desktop layout — persistent sidebar (collapsible),
  full topbar, main content.
- `< md` (<768px): mobile layout — no persistent sidebar (Sheet drawer),
  topbar adapts (hamburger + search can collapse into icon), main content
  takes full width.

## Acceptance Criteria

- [ ] `npm run build` exits 0.
- [ ] `npm run typecheck` exits 0.
- [ ] All 6 nav items have lucide icons and working `NavLink` links.
- [ ] Sidebar collapses to icon-only mode; state persists across reload.
- [ ] On viewport `<768px`, sidebar is hidden; hamburger opens Sheet drawer.
- [ ] Topbar shows target name badge, capture status dot + label, and search
      input placeholder.
- [ ] Theme toggle (sun/moon) is accessible in the topbar on all viewports.
- [ ] Right-side detail Sheet exists, opens/closes via `useDetailDrawer()`
      hook, and does not overlay main content on desktop.
- [ ] Layout does not break on mobile (no horizontal overflow, nav is
      reachable).
- [ ] Dark mode renders correctly in both desktop and mobile layouts.
- [ ] No `fetch`/XHR/WebSocket calls (064 decoupling preserved).

## Out of Scope

- Real target data wiring (comes with 066 API client + backend config).
- Real capture status toggle (comes with WebSocket integration).
- Global search handler (comes with SQL List filter wiring).
- Detail drawer content (comes with SQL Detail/Connection Detail features).
- Mobile filter drawer (comes with SQL List mobile view).
- Mobile SQL cards (comes with SQL List mobile view).

## Constraints

- Frontend standalone; no Rust changes; no backend coupling.
- Use shadcn components from 065 (Sheet, Badge, Input, Button, Tooltip).
- Lucide icons already installed (`lucide-react` from 065).
- CSS approach: Tailwind responsive utilities (`md:` prefix) for breakpoint
  handling. No CSS-in-JS, no media query JS libs.
