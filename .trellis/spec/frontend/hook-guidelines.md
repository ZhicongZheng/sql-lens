# Hook Guidelines

> Hook conventions for the planned SQL Lens frontend.

## Overview

The frontend is not implemented yet. These rules define the intended hook
contracts for React, TypeScript, TanStack Query, and WebSocket-driven SQL event
inspection.

## Custom Hook Patterns

- Hook names must start with `use`.
- Keep data-fetching hooks close to the feature that owns the screen.
- Keep generic API/WebSocket primitives under `web/src/lib/api` and
  `web/src/lib/websocket`.
- Return stable, typed objects from hooks instead of positional tuples unless
  matching an established library convention.
- Keep transient component-only state inside components until a second component
  needs the same behavior.

## Data Fetching

- TanStack Query owns REST server state.
- Query keys should include durable filter values so cached pages are tied to the
  visible URL/query state.
- Mutating or replay actions should invalidate or update only the affected query
  keys.
- WebSocket hooks should update the TanStack Query cache through explicit event
  handlers rather than maintaining a parallel global event store.

## URL And Live State

- URL state owns filters that should survive reloads or sharing.
- Local component state owns temporary controls such as open menus, selected
  tabs, draft filter text, and paused live-stream toggles.
- Live streams should support pausing or buffering so rows do not jump while a
  user is reading details.

## Naming Conventions

- Use domain names: `useSqlEvents`, `useSqlEventDetail`, `useConnections`,
  `useStatistics`, `useSqlStream`.
- Use `useSomethingQuery` only when the hook is a thin TanStack Query wrapper.
- Use `useSomethingSubscription` for WebSocket subscription hooks.

## Tests Required

For hook changes:

- Query key tests or component tests proving filters affect server-state cache
  correctly.
- WebSocket subscription tests for connect, message, pause/resume, and cleanup.
- Tests for URL state parsing when hooks read durable filters.

## Common Mistakes

- Do not store server state in React local state when TanStack Query should own
  it.
- Do not create a global client store before a real cross-feature need exists.
- Do not duplicate API payload decoding across multiple hooks.
- Do not leave WebSocket listeners active after unmount.
