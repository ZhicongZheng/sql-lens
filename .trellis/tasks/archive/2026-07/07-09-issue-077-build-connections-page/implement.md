# Implementation Plan: Build Connections page (Issue 077)

## Overview
Ordered checklist for implementing the Connections page, with validation commands and review gates.

## Prerequisites
- [ ] Verify Issue 030 (connections endpoints) is implemented and API contract is stable
- [ ] Verify Issue 068 (app layout shell) has Connections nav link wired to `/connections`
- [ ] Verify Issue 066 (API client) exports `getConnections()` and connection types
- [ ] Review `prd.md` and `design.md` for alignment

## Phase 1: Setup & Types

### 1.1 Create directory structure
```bash
mkdir -p src/features/connections/{components,hooks,types}
touch src/features/connections/{ConnectionsPage.tsx,index.ts}
touch src/features/connections/components/{ConnectionTable.tsx,ConnectionFilters.tsx,ConnectionRow.tsx}
touch src/features/connections/hooks/useConnections.ts
```

### 1.2 Define or import connection types
- Check if `src/types/connection.ts` or API client already defines `ConnectionInfo`
- If not, create type definition matching API contract from Issue 030
- Export from `src/features/connections/types/index.ts`

**Validation**: `npx tsc --noEmit` passes with no connection type errors

## Phase 2: Data Layer

### 2.1 Implement `useConnections` hook
File: `src/features/connections/hooks/useConnections.ts`

- Wrap `getConnections(filter)` with `useQuery`
- Query key: `['connections', filter]`
- Stale time: 30 seconds
- Return typed data, loading, error states

**Validation**:
```bash
npm run typecheck
# Expected: No errors in useConnections.ts
```

### 2.2 Add MSW mock (optional, for testing)
- If project uses MSW for API mocking, add connections handler
- Mock both active and closed connection lists

## Phase 3: UI Components

### 3.1 Implement `ConnectionFilters`
File: `src/features/connections/components/ConnectionFilters.tsx`

- Render active/closed toggle (Tabs or ToggleGroup from shadcn/ui)
- Accept `value` and `onChange` props
- Default to `active`

**Validation**:
```bash
npm run lint -- src/features/connections/components/ConnectionFilters.tsx
```

### 3.2 Implement `ConnectionRow`
File: `src/features/connections/components/ConnectionRow.tsx`

- Render single table row with all columns from UI.md
- Format timestamps (relative time or absolute)
- Format bytes (human-readable, e.g., "1.2 MB")
- Apply state badge (green for active, gray for closed)
- Attach click handler prop

**Validation**:
```bash
npm run typecheck
npm run lint -- src/features/connections/components/ConnectionRow.tsx
```

### 3.3 Implement `ConnectionTable`
File: `src/features/connections/components/ConnectionTable.tsx`

- Render `<Table>` with header row matching columns
- Map connections to `<ConnectionRow>`
- Pass `onRowClick` handler
- Handle empty state (no connections for current filter)
- Handle loading state (skeleton rows)

**Validation**:
```bash
npm run typecheck
```

### 3.4 Implement responsive mobile view (stretch)
- If time permits, add mobile card list view
- Or document as follow-up

## Phase 4: Page Assembly

### 4.1 Implement `ConnectionsPage`
File: `src/features/connections/ConnectionsPage.tsx`

- Compose: `ConnectionFilters` + `ConnectionTable`
- Manage filter state (local + URL sync)
- Wire `useConnections(filter)` hook
- Handle loading/error/empty at page level or delegate to table
- Set page title or document metadata if applicable

**Validation**:
```bash
npm run typecheck
npm run lint -- src/features/connections/ConnectionsPage.tsx
```

### 4.2 Wire route
- Ensure React Router has route `/connections` → `ConnectionsPage`
- Verify sidebar nav link from Issue 068 layout points to correct route

**Validation**:
```bash
npm run dev
# Manual: Click "Connections" in sidebar, expect page renders
```

## Phase 5: Polish & States

### 5.1 Error state
- Display error banner on API failure
- Include retry button that refetches query

### 5.2 Empty state
- Show friendly message when filter returns zero results
- Different message for "active" vs "closed"

### 5.3 Loading state
- Skeleton table rows while query is pending
- Or spinner + "Loading connections..." text

**Validation**:
```bash
# Simulate slow network or API error via devtools / MSW
npm run dev
```

## Phase 6: Testing

### 6.1 Unit tests (if project has test setup)
- Test `ConnectionFilters` toggle behavior
- Test `ConnectionRow` renders all fields correctly
- Test `useConnections` hook with mocked API

### 6.2 Integration test (optional)
- Render `ConnectionsPage` with MSW, verify table populates

**Validation**:
```bash
npm test -- --testPathPattern=connections
```

## Phase 7: Documentation & Handoff

### 7.1 Update any relevant docs
- If `AGENTS.md` or component catalog exists, add Connections page entry
- No need to update `UI.md` (already documents the page)

### 7.2 Self-review checklist
- [ ] All ACs from `prd.md` covered
- [ ] No console errors in dev mode
- [ ] Dark mode renders correctly
- [ ] Mobile viewport does not break (at minimum, horizontal scroll on table)
- [ ] Filter state persists in URL

## Rollback Plan
If critical issues found post-merge:
1. Revert PR
2. Connections nav link can be hidden or point to 404 temporarily
3. No data loss risk (read-only page)

## Estimated Effort
- Per Issue 077: 5 hours
- Breakdown:
  - Setup & types: 30 min
  - Data layer: 1 hour
  - Components: 2 hours
  - Page assembly: 1 hour
  - Polish & states: 30 min
  - Testing: (variable, not included in estimate)
