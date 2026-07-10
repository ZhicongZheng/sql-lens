# Build Statistics page

## Goal

Implement a Statistics page that visualizes query volume, latency percentiles, error rates, and top fingerprints using ECharts, with a time window selector to update displayed data.

## Background

- Issue 078 requests a Statistics page (P2 priority, 6h estimated).
- `UI.md` (lines 53-71) defines Dashboard widgets including QPS, latency p50/p95/p99, active connections, slow/error SQL, protocol mix, top fingerprints, and error timeline; Statistics page extends this with ECharts visualizations.
- Backend statistics endpoint (`GET /api/v1/statistics`) is defined in Issue 031 with window parameter validation.
- Current implementation (`src/app/routes/statistics.tsx`) uses a `PageStub` placeholder.
- Statistics page is part of the main navigation (Issue 068) alongside Dashboard, SQL, Connections, Replay, Settings.
- ECharts is the mandated charting library per Issue 078 acceptance criteria.

## Requirements

- Display charts for query volume (QPS over time), latency percentiles (p50, p95, p99), error rates, and top fingerprints.
- Use ECharts for all chart rendering.
- Implement a time window selector (e.g., last 1h, 6h, 24h, 7d) that updates chart data.
- Handle empty state when no statistics data is available.
- Follow existing frontend patterns: TypeScript, shadcn/ui components, Tailwind styling, dark mode support.
- Match navigation structure from AppShell/Sidebar.

## Acceptance Criteria

- [ ] StatisticsRoute renders without using PageStub placeholder.
- [ ] Charts are rendered using ECharts for: query volume, latency percentiles, error rate, top fingerprints.
- [ ] Time window selector is present and updates displayed data when changed.
- [ ] Empty state is handled (no data available).
- [ ] Page follows existing UI conventions (consistent with Dashboard, SQL List, Connections pages).
- [ ] Dark mode styling is consistent with other pages.
- [ ] No console errors or TypeScript compilation errors.

## Out of Scope

- Backend statistics API implementation (assumed provided by Issue 031).
- Real-time data streaming or WebSocket integration for live statistics.
- Exporting chart data or images.
- Custom chart interactions beyond time window selection.
- Fingerprint detail drill-down (clicking fingerprints to open SQL List filters).

## Technical Notes

- Reference: `UI.md` lines 53-71 for Dashboard widget definitions and time window selector pattern.
- Reference: Issue 031 for statistics API contract (QPS, error rate, slow count, latency percentiles, active connections, window parameter).
- Existing patterns: `src/features/connections/`, `src/app/routes/sql-events.tsx` for page structure.
- Component library: shadcn/ui components already available in `src/components/ui/`.
- Charting library: ECharts must be used (per Issue 078).
