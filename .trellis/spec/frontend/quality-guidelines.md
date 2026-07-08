# Frontend Quality Guidelines

> Quality standards for the planned SQL Lens frontend.

## Overview

The `web/` app is not present yet. When it is added, it should behave like a
developer debugging tool: fast to scan, safe with untrusted SQL text, and stable
under live updates.

## Code Quality

- Use TypeScript and avoid `any`.
- Keep feature code under `web/src/features/*`.
- Keep shared UI, API, formatting, and websocket helpers in their documented
  directories.
- Prefer small, explicit components and hooks over broad generic abstractions.
- Add dependencies only when they are part of the planned stack or justified by
  the task.

## User Experience Quality

- Optimize for inspection workflows: filtering, comparison, details, replay
  confirmation, and pause/resume should be easy to reach.
- Avoid marketing-style hero pages for the application shell.
- Tables, timelines, charts, and detail panels should use stable dimensions so
  live data does not cause distracting layout shifts.
- Display SQL and database errors as escaped text.

## Accessibility

- Provide accessible labels for icon buttons and controls.
- Keep keyboard navigation usable for tables, dialogs, tabs, and filters.
- Do not rely on color alone for query status.
- Confirmation dialogs are required for replay or other mutating actions.

## Testing Requirements

Before merging UI work:

- Component tests for logic-heavy controls and state transitions.
- Playwright smoke tests for major flows once routing exists.
- XSS tests for SQL text, parameters, and database error rendering.
- Visual screenshots in PRs for significant UI changes.
- Frontend type-check and lint commands once the toolchain exists.

## Security Checks

- Treat SQL text, parameter values, database names, usernames, client addresses,
  and database error messages as untrusted display text.
- Do not render API-provided strings with `dangerouslySetInnerHTML`.
- Replay actions need explicit confirmation and clear target information.
- Do not store credentials or secrets in URL state.

## Common Mistakes

- Do not create a landing page instead of the actual tool surface.
- Do not duplicate backend filter semantics in multiple frontend locations.
- Do not let websocket updates mutate paused or unrelated views.
- Do not add charts that cannot be traced back to typed backend statistics.
