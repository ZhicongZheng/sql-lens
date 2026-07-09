# Build SQL Detail page (Issue 074)

## Goal
Implement the SQL Detail page for viewing comprehensive information about individual SQL events captured by SQL Lens, including original/expanded SQL, parameters, timings, and connection metadata.

## Requirements
- Display SQL event detail with all sections documented in UI.md:
  - Summary (timestamp, protocol, database, user, client/backend addresses, duration, status, rows)
  - Original SQL (with Monaco Editor read-only)
  - Expanded SQL (with Monaco Editor, toggle between original/expanded)
  - Parameters table (index, name, type, value, redaction state)
  - Timings (duration breakdown if available)
  - Result (affected/returned rows)
  - Error (error code, SQLSTATE, message if status is error)
  - Connection (link to connection detail or embedded connection info)
  - Protocol metadata (protocol-specific fields)
  - Replay section (preview button, future execute capability)
- Support navigation from SQL List page (row click or detail button)
- Handle missing event (404/not found state)
- Support copy actions for SQL text
- Follow existing frontend patterns (TanStack Query, shadcn/ui, TypeScript, React Router)
- Support responsive design (desktop side-by-side or stacked, mobile stacked)
- Support light/dark theme with Monaco Editor theme sync
- Integrate with existing API client (`getSqlEvent(id)` from Issue 066)

## Acceptance Criteria
- [ ] SQL Detail page renders at route `/sql-events/{id}`
- [ ] All UI.md documented sections are present and populated
- [ ] Monaco Editor displays original and expanded SQL with syntax highlighting
- [ ] Toggle between original/expanded SQL works
- [ ] Parameter table renders with correct type, value, redaction indicators
- [ ] Binary parameter summaries displayed safely (no raw binary)
- [ ] Error section only visible when status is error, with proper error details
- [ ] Connection section shows connection info with link to Connections page
- [ ] Copy button for SQL text works (clipboard API)
- [ ] Loading state displays during API fetch
- [ ] Not found state displays when event ID does not exist
- [ ] Error state displays on API failure
- [ ] Navigation from SQL List (Issue 070) to detail works
- [ ] Browser back navigation returns to SQL List with preserved state
- [ ] Component follows frontend directory structure and coding guidelines
- [ ] Works on both desktop and mobile viewports

## Dependencies
- Issue 029: SQL event detail endpoint `GET /api/v1/sql-events/{id}` (required)
- Issue 070: Build SQL List page (required, provides navigation entry point)
- Issue 066: Frontend API client with `getSqlEvent()` (required)
- Issue 076: Build parameter table component (P1, may be reused or reimplemented)
- Issue 075: Integrate Monaco SQL viewer (P1, may be implemented together or separately)

## Constraints
- Must not implement replay execute (Issue 080/081 are separate)
- Must not implement connection detail page (separate issue)
- PRD-only is insufficient; design.md and implement.md required before `task.py start` due to component complexity, Monaco integration, and multi-section layout
