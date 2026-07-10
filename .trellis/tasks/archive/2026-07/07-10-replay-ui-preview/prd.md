# Add replay UI preview

## Goal

Implement a Replay preview UI that displays target connection, SQL preview, and mutation risk classification, with clear safety warnings for mutating SQL and an explicit confirmation mechanism before execution.

## Background

- Issue 081 requests Replay UI preview (P2 priority, 5h estimated).
- `UI.md` (lines 173-184) defines Replay requirements: show target/SQL, warn on mutations, require explicit confirmation, show results, keep replay history separate.
- Backend replay preview API (`POST /api/v1/replay/preview`) is implemented in Issue 080 with `ReplayPreviewRequest` (event_id or sql) and `ReplayPreviewResponse` (source, event_id, sql, is_mutation, warning).
- Current implementation (`src/app/routes/replay.tsx`) uses a `PageStub` placeholder.
- Replay page is part of the main navigation (Issue 068) alongside Dashboard, SQL, Connections, Statistics, Settings.
- Issue 081 acceptance criteria explicitly states: "Execute button can remain disabled until execution endpoint exists" — indicating execution is out of scope for this task.

## Requirements

- Display target connection or configured replay target.
- Show SQL preview (from event or raw SQL input).
- Classify and display mutation risk (is_mutation flag from API).
- Show clear warning message for mutating SQL.
- Handle both event-based preview (via event_id) and raw SQL preview.
- Follow existing frontend patterns: TypeScript, shadcn/ui components, Tailwind styling, dark mode support.
- Match navigation structure from AppShell/Sidebar.

## Acceptance Criteria

- [ ] ReplayRoute renders without using PageStub placeholder.
- [ ] Preview UI shows target, SQL, and risk classification (is_mutation).
- [ ] Mutating SQL displays explicit warning message.
- [ ] Execute button is present but disabled (per Issue 081: "can remain disabled until execution endpoint exists").
- [ ] Page follows existing UI conventions (consistent with Dashboard, SQL List, Connections pages).
- [ ] Dark mode styling is consistent with other pages.
- [ ] No console errors or TypeScript compilation errors.

## Out of Scope

- Actual SQL execution functionality (Issue 081 explicitly allows disabled Execute button).
- Replay history tracking or storage.
- Result display after execution.
- Multiple target selection UI.
- Integration with real replay execution endpoint.

## Technical Notes

- Reference: `UI.md` lines 173-184 for Replay UI requirements and safety messaging.
- Reference: Issue 080 backend API contract (`ReplayPreviewRequest`, `ReplayPreviewResponse`, `MUTATION_WARNING` constant).
- Existing patterns: `src/features/connections/`, `src/app/routes/sql-events.tsx` for page structure.
- Component library: shadcn/ui components already available in `src/components/ui/`.
- API client: Use existing pattern from `src/lib/api/` for calling replay preview endpoint.
