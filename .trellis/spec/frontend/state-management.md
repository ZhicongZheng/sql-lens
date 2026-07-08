# State Management

> State ownership rules for the planned SQL Lens frontend.

## Overview

SQL Lens frontend state should be explicit and split by ownership. The current
backend already exposes REST and WebSocket surfaces; the future UI should avoid
duplicating server data in local stores.

## State Categories

- Server state: SQL events, connections, protocol list, statistics, health, and
  event details. Owned by TanStack Query.
- URL state: durable filters, selected timeline cursor, search text, and view
  modes that should survive refresh or sharing.
- Local component state: open menus, active tabs, selected rows, modal state,
  draft input text, and pause/resume toggles.
- Derived state: computed from server, URL, or local state during render unless
  it is expensive enough to memoize.

## When To Use Global State

Do not introduce a global client state library by default. Promote state beyond a
component or feature only when:

- At least two independent feature areas need to read and write it.
- URL state is not suitable because the value is temporary or sensitive.
- TanStack Query is not suitable because the value is not server state.

Document the reason in the feature task before adding a new global state
dependency.

## Server State

- Use TanStack Query for REST data fetching, caching, refetching, and invalidation.
- Query keys must include filter and pagination inputs.
- WebSocket updates should flow into query cache updates or explicit live buffers.
- Do not let live updates mutate a detail view for a different selected event.

## URL State

- Put shareable filters in the URL, not only in memory.
- Keep backend query parameter names aligned with API contracts such as
  `limit`, `cursor`, `protocol`, `database_type`, `status`, `q`, `from`, and
  `to`.
- Parse URL values into typed frontend filter objects before passing them to API
  hooks.

## Tests Required

For state-management changes:

- Tests that URL filters survive reload-like reinitialization.
- Tests that TanStack Query keys include all server-state inputs.
- Tests that WebSocket updates do not reorder or overwrite paused views.

## Common Mistakes

- Do not mirror every API response into a hand-written store.
- Do not keep durable filters only in component state.
- Do not use `any` when moving state between URL, API hooks, and components.
- Do not create hidden state coupling between dashboard charts and event tables.
