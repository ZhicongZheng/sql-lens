# Design: Build Connections page (Issue 077)

## Overview
This document describes the technical design for the Connections page, including component architecture, data flow, state management, and integration with existing frontend systems.

## Architecture

### Component Structure
```
src/
  features/
    connections/
      components/
        ConnectionTable.tsx          # Main table component
        ConnectionFilters.tsx        # Active/closed filter controls
        ConnectionRow.tsx            # Individual row (clickable)
      hooks/
        useConnections.ts            # TanStack Query hook for connections API
      types/
        index.ts                     # Re-export connection types
      ConnectionsPage.tsx            # Page container
```

### Data Flow
1. **API Layer**: Use existing API client from Issue 066 (`getConnections`, `getConnectionDetail`)
2. **Query Hook**: `useConnections(filter: 'active' | 'closed' | 'all')` wraps TanStack Query
3. **Page Component**: Manages filter state, renders table + filters
4. **Table Component**: Renders rows, handles row click navigation
5. **Navigation**: Row click triggers React Router navigation to `/connections/{id}` (detail page stubbed or implemented separately)

### State Management
- **Server State**: TanStack Query manages connection list data
  - Query key: `['connections', filter]`
  - Stale time: 30 seconds (connections change moderately)
  - Refetch on window focus: enabled
- **Local State**: Filter selection (`active` | `closed`)
  - Default: `active` (most relevant view)
  - Persist in URL query param `?state=active|closed` for shareability

### API Integration
- Endpoint: `GET /api/v1/connections?state=active|closed`
- Response shape: Array of `ConnectionInfo` (from Issue 030 API contract)
- Error handling: Map API errors to user-friendly messages
- Loading state: Show skeleton table rows during fetch

### UI Components (shadcn/ui)
- **Table**: `@/components/ui/table`
- **Tabs or ToggleGroup**: For active/closed filter
- **Badge**: For connection state (active=green, closed=gray)
- **Skeleton**: Loading placeholder
- **Empty state**: Custom message with icon

### Responsive Behavior
- **Desktop (≥768px)**: Full table with all columns
- **Mobile (<768px)**: Card-based list (per UI.md pattern)
  - Each card shows: ID, Protocol, Client→Backend, State, Duration
  - Tap card to open detail

### Theme Support
- Use Tailwind dark mode classes
- Status colors per UI.md:
  - Active: green (`text-green-600`, `bg-green-100`)
  - Closed: neutral (`text-gray-600`, `bg-gray-100`)
- Ensure SQL/connection text remains readable in dark mode

### Navigation Integration
- Sidebar link "Connections" (from Issue 068 layout) routes to `/connections`
- Row click: `navigate(`/connections/${connectionId}`)`
- Back navigation: Browser back or "Back to Connections" link on detail page

## Data Model
Connection fields (from UI.md):
```typescript
interface ConnectionInfo {
  id: string;
  protocol: string;
  client_addr: string;
  backend_addr: string;
  user: string;
  database: string;
  state: 'active' | 'closed';
  connected_at: string;      // ISO timestamp
  last_activity_at: string;  // ISO timestamp
  query_count: number;
  bytes_in: number;
  bytes_out: number;
}
```

## File Locations
- Page: `src/features/connections/ConnectionsPage.tsx`
- Components: `src/features/connections/components/`
- Hook: `src/features/connections/hooks/useConnections.ts`
- Types: Re-export from `src/types/connection.ts` (or API client types)

## Error Scenarios
1. **API failure**: Display error banner with retry button
2. **Empty result**: "No active connections" or "No closed connections" message
3. **Network timeout**: TanStack Query retry (default 3 attempts)

## Testing Strategy
- Unit: Filter toggle logic, row click handler
- Integration: API hook with MSW mock
- E2E: Navigate to Connections, toggle filter, click row (if detail exists)
