# Design — Issue 068: App layout shell

## Current state

- `app-shell.tsx`: flex row, sidebar + (topbar + main/Outlet). No responsive.
- `sidebar.tsx`: 6 text-only `NavLink` items in `NAV_ITEMS` array, fixed
  `w-56`, no icons, no collapse.
- `topbar.tsx`: `<Placeholder />` stubs for target, search, controls. Theme
  toggle button present (Sun/Moon icons).

## Architecture

### Sidebar state management

A `SidebarProvider` context (new, in `src/app/providers/sidebar-provider.tsx`)
owns the collapse state:

- `isCollapsed: boolean` — desktop collapse mode.
- `isMobileOpen: boolean` — mobile Sheet open state.
- `toggleCollapse()` — desktop toggle.
- `openMobile()` / `closeMobile()` — mobile Sheet control.
- Collapse persists to `localStorage` key `sql-lens-sidebar-collapsed`.

The provider wraps the app shell. Both the sidebar and topbar consume it.

### Sidebar component refactor

`sidebar.tsx` becomes the nav list component, used in two places:

1. Desktop: rendered directly in `app-shell.tsx`, hidden on `< md` via
   `hidden md:flex`.
2. Mobile: rendered inside a shadcn `Sheet` (left side), triggered by
   hamburger button in topbar, visible only on `< md`.

The nav list itself is extracted as `SidebarNav` (the items + icons), used by
both the desktop sidebar and the mobile Sheet. This avoids duplicating nav
items.

### Topbar refactor

`topbar.tsx` layout:

```
[hamburger(mobile)] [target badge] [capture status] [spacer] [search input] [theme toggle]
```

- Hamburger: `Button variant=ghost size=icon` with `Menu` icon, visible
  `md:hidden`, calls `openMobile()`.
- Target badge: `Badge variant=outline` showing target name (hardcoded
  `mysql-local` placeholder).
- Capture status: dot (`span` with `bg-status-*` + `rounded-full`) + label
  text. Default `active` → `text-status-ok`.
- Search: `Input` with `Search` icon left-adornment, `placeholder="Search
  SQL..."`.
- Theme toggle: existing Sun/Moon button, moved to far right.

### Detail drawer

A new `DetailDrawer` component in `src/components/layout/detail-drawer.tsx`.
It renders a shadcn `Sheet` on the right side (`side="right"`).

Context in `src/app/providers/detail-drawer-provider.tsx`:
- `isOpen: boolean`, `openDrawer()`, `closeDrawer()`.
- Default: closed. No route binding — opened programmatically by feature
  components later.

In `app-shell.tsx`, the detail drawer is a flex sibling of the main content
area (not an overlay). When open, the main content compresses via flex.

### Responsive strategy

All breakpoint handling uses Tailwind's `md:` prefix (≥768px). No JS
`matchMedia` listener — CSS handles visibility. The `Sheet` for mobile nav
is always in the DOM but only visible when triggered.

Desktop (≥md):
```
[Sidebar 224px/64px] [Topbar 48px + Main content (flex-1)] [DetailDrawer?]
```

Mobile (<md):
```
[Topbar 48px (hamburger + badge + status + search + theme)]
[Main content (full width)]
[Sheet left: mobile nav]
[Sheet right: detail drawer]
```

## File changes

| File | Change |
|---|---|
| `src/app/providers/sidebar-provider.tsx` | **new** — SidebarProvider context |
| `src/app/providers/detail-drawer-provider.tsx` | **new** — DetailDrawer context |
| `src/components/layout/sidebar.tsx` | refactor — extract SidebarNav, icons, collapse, responsive |
| `src/components/layout/topbar.tsx` | rewrite — target badge, status, search, hamburger, theme toggle |
| `src/components/layout/app-shell.tsx` | update — responsive classes, detail drawer, providers |
| `src/components/layout/detail-drawer.tsx` | **new** — right-side Sheet placeholder |
| `src/main.tsx` | add SidebarProvider + DetailDrawerProvider wrappers |
| `src/app/routes/dashboard.tsx` | minor — remove import of old `PageStub` if needed |

## Non-goals

- No real data wiring. No WebSocket. No API calls. No filter handlers.
- No mobile SQL cards or mobile filter drawer (SQL List feature).
- No Rust changes.
