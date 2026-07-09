# Implement — Issue 068: App layout shell

## 1. SidebarProvider

- [ ] Create `src/app/providers/sidebar-provider.tsx`:
      - `isCollapsed`, `isMobileOpen` state.
      - `toggleCollapse()`, `openMobile()`, `closeMobile()`.
      - Persist `isCollapsed` to `localStorage` key `sql-lens-sidebar-collapsed`.
      - Initial: read stored value, default `false`.
- [ ] Wrap `<App>` in `<SidebarProvider>` in `src/main.tsx`.

## 2. DetailDrawerProvider

- [ ] Create `src/app/providers/detail-drawer-provider.tsx`:
      - `isOpen` state, `openDrawer()`, `closeDrawer()`.
- [ ] Wrap `<App>` in `<DetailDrawerProvider>` in `src/main.tsx` (inside
      SidebarProvider, outside BrowserRouter).

## 3. Sidebar refactor

- [ ] Extract `SidebarNav` component (the nav list + icons) into
      `src/components/layout/sidebar-nav.tsx`:
      - `NAV_ITEMS` with icon component (from lucide-react).
      - `collapsed` prop: when true, render icon-only tooltips.
      - Each item: icon + label (hidden when collapsed).
      - Active state highlight via `NavLink` className callback.
- [ ] Refactor `sidebar.tsx`:
      - Desktop wrapper with collapse toggle button at the top.
      - Width: `w-56` expanded, `w-16` collapsed.
      - Uses `useSidebar()` for collapse state.
      - Hidden on `< md` via `hidden md:flex`.
- [ ] Add collapse toggle button with `PanelLeftClose`/`PanelLeft` icons,
      `aria-label="Toggle sidebar"`.

## 4. Topbar rewrite

- [ ] Rewrite `topbar.tsx`:
      - Hamburger button (mobile only, `md:hidden`), calls `openMobile()`.
      - Target badge: `<Badge variant="outline">mysql-local</Badge>`.
      - Capture status: green dot + "Active" label (`text-status-ok`).
      - Search input: `<Input>` with `Search` icon, placeholder.
      - Theme toggle: existing Sun/Moon button, right-aligned.
      - Use `flex items-center gap-2 h-12 px-4 border-b`.

## 5. Mobile Sheet nav

- [ ] In `app-shell.tsx` (or a new `MobileNav` component):
      - Render `<Sheet side="left">` containing `<SidebarNav collapsed={false}
        onNavigate={closeMobile} />`.
      - Controlled by `isMobileOpen` from `useSidebar()`.
      - Only rendered (or only visible) on `< md` via CSS.

## 6. Detail drawer

- [ ] Create `src/components/layout/detail-drawer.tsx`:
      - `<Sheet side="right">` controlled by `useDetailDrawer().isOpen`.
      - Placeholder content: "Detail panel — content arrives with SQL Detail".
      - `sm:max-w-lg` width.
- [ ] In `app-shell.tsx`, render `<DetailDrawer />` as a flex sibling of the
      main content area (not overlay).

## 7. AppShell responsive refactor

- [ ] Update `app-shell.tsx`:
      - Desktop: `[Sidebar] [Topbar+Main flex-col] [DetailDrawer?]`.
      - Mobile: `[Topbar] [Main]` (sidebar via Sheet).
      - Use Tailwind responsive classes for sidebar visibility.

## 8. Main.tsx provider wiring

- [ ] Confirm provider nesting order:
      `StrictMode > ThemeProvider > SidebarProvider > DetailDrawerProvider >
      TooltipProvider > BrowserRouter > App + Toaster`.

## Validation gates

- [ ] `npm run build` → exit 0.
- [ ] `npm run typecheck` → exit 0.
- [ ] `grep -rnE "fetch\(|XMLHttpRequest|new WebSocket" src/` → no matches.
- [ ] `grep -rn "/api/v1" src/` → no matches.
- [ ] Desktop: sidebar visible, collapsible, icons + labels.
- [ ] Mobile: sidebar hidden, hamburger visible, Sheet opens.
- [ ] Topbar: target badge + capture status + search + theme toggle.
- [ ] Detail drawer opens/closes, compresses main on desktop.
- [ ] Dark mode: both desktop and mobile render correctly.

## Rollback

All changes in `crates/sql-lens-app/web/src/`. `git checkout -- src/` reverts.
