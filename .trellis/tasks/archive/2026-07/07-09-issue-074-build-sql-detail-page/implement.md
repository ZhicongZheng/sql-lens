# Implementation Plan: Build SQL Detail page (Issue 074)

## Overview
Ordered checklist for implementing the SQL Detail page, with validation commands and review gates.

## Prerequisites
- [ ] Verify Issue 029 (SQL event detail endpoint) is implemented and returns full `SqlEvent` schema
- [ ] Verify Issue 070 (SQL List page) has navigation entry point (row click or detail action)
- [ ] Verify Issue 066 (API client) exports `getSqlEvent(id: string)`
- [ ] Check if Monaco Editor (`@monaco-editor/react`) is already installed
- [ ] Review `prd.md` and `design.md` for alignment

## Phase 1: Setup & Exploration

### 1.1 Verify dependencies
```bash
cd crates/sql-lens-app/web
npm ls @monaco-editor/react
# If not installed:
npm install @monaco-editor/react
```

### 1.2 Explore existing SQL List implementation
- Read `src/app/routes/sql-events.tsx` to understand navigation pattern
- Read `src/lib/api/hooks/use-sql-events.ts` for query patterns
- Identify how row click currently works (if at all)

**Validation**: Understand entry point for detail navigation

## Phase 2: Data Layer

### 2.1 Implement `useSqlEvent` hook
File: `src/features/sql-events/hooks/useSqlEvent.ts`

```typescript
import { useQuery } from "@tanstack/react-query";
import { getSqlEvent } from "@/lib/api/client";

export function useSqlEvent(id: string) {
  return useQuery({
    queryKey: ["sql-event", id],
    queryFn: () => getSqlEvent(id),
    enabled: !!id,
    staleTime: 5 * 60 * 1000, // 5 minutes
  });
}
```

**Validation**:
```bash
npm run typecheck
# Expected: No errors in useSqlEvent.ts
```

### 2.2 Verify API client method
- Check `src/lib/api/client.ts` for `getSqlEvent` implementation
- If missing, add:
```typescript
export async function getSqlEvent(id: string): Promise<SqlEvent> {
  return request<SqlEvent>(`/sql-events/${encodeURIComponent(id)}`);
}
```

**Validation**:
```bash
npm run typecheck
```

## Phase 3: Core Components

### 3.1 Implement `SqlMonacoViewer`
File: `src/features/sql-events/components/SqlMonacoViewer.tsx`

- Wrap `<Editor>` from `@monaco-editor/react`
- Props: `sql: string`, `readOnly?: boolean`
- Theme: sync with app theme context (use `useTheme` or similar)
- Options: `{ readOnly: true, minimap: { enabled: false }, scrollBeyondLastLine: false }`
- Height: fixed (e.g., `200px`) or auto with max-height + scroll

**Validation**:
```bash
npm run typecheck
npm run dev
# Manual: Verify SQL renders with syntax highlighting
```

### 3.2 Implement `SqlSummary`
File: `src/features/sql-events/components/SqlSummary.tsx`

- Display: timestamp, protocol, database_type, user, database, client_addr, backend_addr, target_name, status (badge), duration_ms, fingerprint
- Format timestamp as local time
- Status badge: green (ok), amber (slow), red (error)

**Validation**:
```bash
npm run typecheck
```

### 3.3 Implement `SqlParameterTable`
File: `src/features/sql-events/components/SqlParameterTable.tsx`

- If Issue 076 parameter table component exists and is reusable, import and use it
- Otherwise, implement table with columns: Index, Name, Type, Value, Redacted
- Handle redacted values (show "REDACTED" or mask)
- Handle binary_summary (show summary text, not raw bytes)
- Handle long values (truncate with expand or tooltip)

**Validation**:
```bash
npm run typecheck
npm run lint -- src/features/sql-events/components/SqlParameterTable.tsx
```

### 3.4 Implement `SqlError`
File: `src/features/sql-events/components/SqlError.tsx`

- Conditional render: only if `status === "error"`
- Display: error code, SQLSTATE (if present), error message
- Style: red-tinted card or alert

**Validation**:
```bash
npm run typecheck
```

### 3.5 Implement `SqlConnectionInfo`
File: `src/features/sql-events/components/SqlConnectionInfo.tsx`

- Display: connection_id, client_addr, backend_addr
- Link: "View Connection" → `navigate(\`/connections/${connection_id}\`)` (if connection detail exists) or just display info
- If connection detail page not ready, link can be disabled or point to Connections list with filter

**Validation**:
```bash
npm run typecheck
```

### 3.6 Implement `SqlReplaySection` (stub)
File: `src/features/sql-events/components/SqlReplaySection.tsx`

- Display: "Replay" heading, description
- Button: "Preview SQL" (enabled) → will integrate with Issue 080
- Button: "Execute" (disabled with tooltip "Coming soon")
- Show target connection info if available

**Validation**:
```bash
npm run typecheck
```

## Phase 4: Page Assembly

### 4.1 Implement `SqlDetailPage`
File: `src/features/sql-events/SqlDetailPage.tsx`

- Compose all section components
- Wire `useSqlEvent(id)` hook
- Handle states:
  - Loading: skeleton placeholders for each section
  - Error: error banner with retry
  - Not found: 404-style message with "Back to SQL List" link
  - Success: render all sections
- "Back to SQL List" button: `navigate("/sql-events")`
- Copy JSON button (optional): copy entire event as JSON

**Validation**:
```bash
npm run typecheck
npm run lint -- src/features/sql-events/SqlDetailPage.tsx
```

### 4.2 Implement route wrapper
File: `src/app/routes/sql-detail.tsx` (or integrate into existing routing)

```typescript
import { useParams } from "react-router-dom";
import { SqlDetailPage } from "@/features/sql-events/SqlDetailPage";

export function SqlDetailRoute() {
  const { id } = useParams<{ id: string }>();
  if (!id) return <div>Invalid event ID</div>;
  return <SqlDetailPage eventId={id} />;
}
```

**Validation**:
```bash
npm run typecheck
```

### 4.3 Wire route in App.tsx or router config
- Add route: `/sql-events/:id` → `SqlDetailRoute`
- Ensure SQL List navigation links to detail (update Issue 070 if needed)

**Validation**:
```bash
npm run dev
# Manual: Click SQL event row in list, expect detail page loads
```

## Phase 5: Polish & Interactions

### 5.1 Toggle original/expanded SQL
- In `SqlDetailPage` or `SqlMonacoViewer` parent, add state: `viewMode: "original" | "expanded"`
- Render toggle buttons or segmented control above Monaco
- Default: "original"
- Only show expanded if `expanded_sql !== original_sql`

### 5.2 Copy SQL functionality
- Add copy button to each Monaco viewer
- On click: `navigator.clipboard.writeText(sql)`
- Show transient toast or button text change ("Copied!")

**Validation**:
```bash
npm run dev
# Manual: Click copy, verify clipboard has SQL text, toast appears
```

### 5.3 Responsive adjustments
- Desktop: sections side-by-side where space allows (summary + SQL can be full-width stacked)
- Mobile: all sections stack, Monaco full-width

**Validation**:
```bash
npm run dev
# Manual: Resize viewport, verify no horizontal overflow
```

## Phase 6: Testing

### 6.1 Unit tests (if project has test setup)
- Test `useSqlEvent` hook with mocked API
- Test toggle logic in SQL viewer
- Test conditional rendering (error section, expanded SQL)

### 6.2 Integration test (optional)
- Render `SqlDetailPage` with MSW mock, verify all sections populate

**Validation**:
```bash
npm test -- --testPathPattern=sql-detail
```

## Phase 7: Documentation & Handoff

### 7.1 Update relevant docs (optional)
- If component catalog or storybook exists, add SqlDetailPage entry
- No need to update `UI.md` (already documents the page)

### 7.2 Self-review checklist
- [ ] All ACs from `prd.md` covered
- [ ] Monaco Editor renders SQL correctly in light/dark mode
- [ ] Navigation from SQL List → Detail → Back works
- [ ] Copy buttons functional
- [ ] No console errors
- [ ] Parameter table handles redaction and binary safely
- [ ] Error section only appears for error status

## Rollback Plan
If critical issues found post-merge:
1. Revert PR
2. SQL List continues to work (detail is additive)
3. Direct URL access to `/sql-events/{id}` will 404 or show stub until re-deploy

## Estimated Effort
- Per Issue 074: 7 hours
- Breakdown:
  - Setup & exploration: 30 min
  - Data layer: 30 min
  - Core components (Monaco, summary, params, error, connection, replay): 3 hours
  - Page assembly: 1.5 hours
  - Polish (toggle, copy, responsive): 1 hour
  - Testing: (variable, not included in estimate)
