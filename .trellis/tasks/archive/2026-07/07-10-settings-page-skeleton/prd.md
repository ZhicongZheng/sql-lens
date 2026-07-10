# Build Settings page skeleton

## Goal

Implement a read-only Settings page skeleton that displays configuration sections for proxy, backend, storage, redaction, slow SQL threshold, plugins, and exporters, following the layout documented in `UI.md`.

## Background

- Issue 079 requests a Settings page skeleton (P2 priority, 5h estimated).
- `UI.md` (lines 186-198) defines 7 configuration sections and requires distinguishing runtime-editable fields from restart-required fields.
- Current implementation (`src/app/routes/settings.tsx`) uses a `PageStub` placeholder.
- Backend configuration is defined in `CONFIG.md` with sections: proxy, backend, tls, web, storage, retention, logging, redaction, replay, plugins.
- Settings page is part of the main navigation (Issue 068) alongside Dashboard, SQL, Connections, Statistics, Replay.

## Requirements

- Display 7 configuration sections as specified in `UI.md`: Proxy, Backend, Storage, Redaction, Slow SQL threshold, Plugins, Exporters.
- Each section should show relevant configuration fields (read-only placeholders acceptable for v1 skeleton).
- Visually mark fields that require restart to take effect.
- Follow existing frontend patterns: TypeScript, shadcn/ui components, Tailwind styling, dark mode support.
- Match navigation structure from AppShell/Sidebar (Dashboard, SQL, Connections, Statistics, Replay, Settings).

## Acceptance Criteria

- [ ] SettingsRoute renders without using PageStub placeholder.
- [ ] All 7 sections from `UI.md` are present: Proxy, Backend, Storage, Redaction, Slow SQL threshold, Plugins, Exporters.
- [ ] Configuration fields are displayed as read-only placeholders (no actual editing required for v1 skeleton).
- [ ] Fields requiring restart are visually distinguished (e.g., marked with icon or label).
- [ ] Page follows existing UI conventions (consistent with Dashboard, SQL List, Connections pages).
- [ ] Dark mode styling is consistent with other pages.
- [ ] No console errors or TypeScript compilation errors.

## Out of Scope

- Actual configuration editing functionality (v1 is read-only skeleton).
- Integration with backend `/api/v1/settings` or config endpoints.
- Real-time config validation or save operations.
- Plugin status or exporter configuration management beyond placeholders.

## Technical Notes

- Reference: `UI.md` lines 186-198 for section definitions and restart field guidance.
- Reference: `CONFIG.md` for configuration field names and structure.
- Existing patterns: `src/features/connections/`, `src/app/routes/sql-events.tsx` for page structure.
- Component library: shadcn/ui components already available in `src/components/ui/`.
- Visual marker for restart-required fields: Use `<Badge variant="outline">Restart required</Badge>` next to field labels, following status badge patterns from `component-guidelines.md`.
