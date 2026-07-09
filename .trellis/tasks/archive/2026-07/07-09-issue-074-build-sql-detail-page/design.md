# Design: Build SQL Detail page (Issue 074)

## Overview
This document describes the technical design for the SQL Detail page, including component architecture, data flow, Monaco Editor integration, and navigation patterns.

## Architecture

### Component Structure
```
src/
  features/
    sql-events/
      components/
        SqlDetailPage.tsx            # Main page container
        SqlSummary.tsx               # Summary metadata section
        SqlMonacoViewer.tsx          # Monaco Editor wrapper (original/expanded toggle)
        SqlParameterTable.tsx        # Parameter display (reuse or extend Issue 076)
        SqlTimings.tsx               # Duration/timing breakdown
        SqlResult.tsx                # Affected/returned rows
        SqlError.tsx                 # Error details (conditional)
        SqlConnectionInfo.tsx        # Connection metadata + link
        SqlProtocolMetadata.tsx      # Protocol-specific fields
        SqlReplaySection.tsx         # Replay preview (stub for Issue 080)
      hooks/
        useSqlEvent.ts               # TanStack Query hook for single event
      SqlDetailRoute.tsx             # Route wrapper with ID param
```

### Data Flow
1. **Route**: `/sql-events/:id` → `SqlDetailRoute` extracts `id` param
2. **Query Hook**: `useSqlEvent(id)` fetches via `getSqlEvent(id)` API client
3. **Page Component**: Renders all sections based on fetched `SqlEvent`
4. **Conditional Sections**:
   - Error section: only if `status === "error"`
   - Expanded SQL: only if `expanded_sql !== original_sql`
   - Replay: always visible (preview enabled, execute disabled until Issue 080)

### State Management
- **Server State**: TanStack Query manages single event data
  - Query key: `["sql-event", id]`
  - Stale time: 5 minutes (events are immutable)
  - Cache time: 30 minutes
- **Local State**:
  - Monaco toggle: `viewMode: "original" | "expanded"` (default: "original")
  - Copy feedback: transient "Copied!" toast on clipboard success

### Monaco Editor Integration
- Use `@monaco-editor/react` (already in project per UI.md)
- Read-only mode: `options={{ readOnly: true }}`
- Theme sync: follow app dark/light mode
  - Dark: `vs-dark`
  - Light: `vs`
- Language: `sql` for all SQL content
- Height: auto-resize or fixed with scroll (e.g., 200px)
- Copy button: positioned absolute top-right, uses `navigator.clipboard.writeText()`

### UI Components (shadcn/ui + custom)
- **Card/Section**: Each major section in a bordered card or `<section>`
- **Badge**: Status (ok=green, slow=amber, error=red), protocol
- **Table**: Parameters (reuse Issue 076 component if compatible)
- **Button**: Copy SQL, "Back to SQL List", "View Connection"
- **Skeleton**: Loading placeholder matching section layout
- **Alert**: Not found / error states

### Navigation
- **Entry**: From SQL List (`/sql-events`), row click or detail icon → `navigate(\`/sql-events/${eventId}\`)`
- **Exit**:
  - Browser back button (React Router handles)
  - "Back to SQL List" link/button → `navigate("/sql-events")`
- **Deep link**: Direct URL access should work (TanStack Query handles fetch)

### Layout (Desktop)
```
┌─────────────────────────────────────────────────────────────┐
│ [← Back]  SQL Event Detail                    [Copy JSON]  │
├─────────────────────────────────────────────────────────────┤
│ Summary Card                                                │
│ Timestamp | Protocol | DB | User | Client→Backend | Status │
├─────────────────────────────────────────────────────────────┤
│ Original SQL                              [Copy] [Toggle ▼] │
│ ┌───────────────────────────────────────────────────────┐  │
│ │ SELECT * FROM users WHERE id = ?                      │  │
│ └───────────────────────────────────────────────────────┘  │
├─────────────────────────────────────────────────────────────┤
│ Expanded SQL (if different)               [Copy]            │
│ ┌───────────────────────────────────────────────────────┐  │
│ │ SELECT * FROM users WHERE id = 123                    │  │
│ └───────────────────────────────────────────────────────┘  │
├─────────────────────────────────────────────────────────────┤
│ Parameters                                                  │
│ ┌───────────────────────────────────────────────────────┐  │
│ │ Index | Name | Type | Value | Redacted                │  │
│ │ 0     | id   | int  | 123   | No                      │  │
│ └───────────────────────────────────────────────────────┘  │
├─────────────────────────────────────────────────────────────┤
│ Timings | Result | Error (if error) | Connection | Meta  │
├─────────────────────────────────────────────────────────────┤
│ Replay (stub)                                               │
│ [Preview] - Shows target SQL, mutation warning              │
└─────────────────────────────────────────────────────────────┘
```

### Layout (Mobile)
- All sections stack vertically
- Monaco Editor: full-width, scrollable
- Tables: horizontal scroll or card layout for parameters

### Theme Support
- Monaco theme switches with app theme context
- Status colors per UI.md (ok=green, slow=amber, error=red)
- Ensure SQL text contrast in both modes

### Error Scenarios
1. **404 Not Found**: Event ID does not exist → friendly "Event not found" with link to SQL List
2. **API Error**: Network/500 → error banner with retry (refetch query)
3. **Malformed Data**: Missing required fields → graceful degradation (show what exists, log warning)

### File Locations
- Page: `src/features/sql-events/SqlDetailPage.tsx`
- Components: `src/features/sql-events/components/`
- Hook: `src/features/sql-events/hooks/useSqlEvent.ts`
- Route: `src/app/routes/sql-detail.tsx` or integrated into existing sql-events routing

## Data Model
Uses existing `SqlEvent` type from `src/types/index.ts` (Issue 066):
```typescript
interface SqlEvent {
  id: string;
  timestamp: string;
  target_name: string;
  protocol: string;
  database_type: string;
  connection_id: string;
  client_addr: string;
  backend_addr: string;
  user: string;
  database: string;
  kind: string;
  status: string;  // "ok" | "slow" | "error"
  duration_ms: number;
  original_sql: string;
  expanded_sql: string;
  fingerprint: string;
  rows: { affected: number; returned: number };
  parameters: SqlParameter[];
  metadata: SqlEventMetadata;
}
```

## Testing Strategy
- Unit: Toggle logic, copy handler, conditional rendering
- Integration: Mock API response, verify all sections render
- E2E: Navigate from SQL List → Detail, verify content, back navigation

## Future Extensions (Out of Scope)
- Issue 075: Monaco integration can be extracted to shared `SqlMonacoViewer` component
- Issue 080: Replay preview will populate the replay section
- Connection detail link will navigate to `/connections/${connection_id}` when Issue 077 detail is ready
