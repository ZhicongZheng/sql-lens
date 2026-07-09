# Build Connections page (Issue 077)

## Goal
Implement the Connections page for viewing active and historical database connections through the SQL Lens proxy, providing visibility into proxy connection lifecycle and state.

## Requirements
- Display connections in a table with columns: Connection ID, Protocol, Client, Backend, User, Database, State, Connected time, Last activity, Query count, Bytes in/out
- Provide filter to toggle between active and closed connections
- Enable row click to open connection detail view
- Handle loading, empty, and error states appropriately
- Follow existing frontend patterns (TanStack Query, shadcn/ui, TypeScript)
- Support responsive design (desktop table, mobile cards)
- Support light/dark theme
- Integrate with existing API client for connections endpoints (Issue 030)

## Acceptance Criteria
- [ ] Connection table renders all documented columns from UI.md
- [ ] Active/closed filter switches connection list view
- [ ] Clicking a row navigates to connection detail
- [ ] Loading state displays during API fetch
- [ ] Empty state displays when no connections match filter
- [ ] Error state displays on API failure
- [ ] Component follows frontend directory structure and coding guidelines
- [ ] Works on both desktop and mobile viewports

## Dependencies
- Issue 030: Connections API endpoints (`GET /api/v1/connections`, `GET /api/v1/connections/{id}`)
- Issue 068: App layout shell with Connections navigation
- Issue 066: Frontend API client with connection types

## Constraints
- Must not implement connection detail page (separate issue)
- Must not implement real-time connection updates via WebSocket (future)
- Must not implement export functionality
- PRD-only is insufficient; design.md and implement.md required before `task.py start` due to component complexity and API integration
