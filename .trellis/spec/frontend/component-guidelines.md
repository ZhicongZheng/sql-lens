# Component Guidelines

> Component conventions for the planned SQL Lens frontend.

## Overview

The `web/` frontend does not exist yet. These rules define the planned React and
TypeScript UI contract from `AGENTS.md` and `CONTRIBUTING.md`; future UI work
should update this file with real component examples as soon as components are
created.

SQL Lens is a developer inspection tool, not a marketing site. Components should
be dense, calm, and optimized for repeated debugging workflows.

## Component Structure

- Use React function components with TypeScript.
- Keep route-level feature composition under `web/src/features/*`.
- Keep reusable primitives and wrappers under `web/src/components/*`.
- Keep `components/ui` for shadcn/ui primitives and thin wrappers.
- Move feature-local components to shared folders only after a second feature
  needs them.
- Prefer small components with clear props over broad generic dashboards.

## Props Conventions

- Define explicit prop types or interfaces next to the component unless the type
  is shared across multiple files.
- Avoid `any`; API payloads should use shared frontend types aligned with backend
  response schemas.
- Keep command callbacks named by user intent, such as `onSelectEvent` or
  `onPauseStream`.
- Use discriminated unions for component modes when a component has mutually
  exclusive states.

## Styling Patterns

- Use TailwindCSS and shadcn/ui as the base styling system.
- Keep tool surfaces compact and predictable: tables, split panes, filters,
  tabs, badges, and detail panels are preferred over decorative cards.
- Use status badges and restrained color to distinguish `ok`, `error`, and
  in-flight states.
- Do not render SQL text, parameters, or database errors as HTML.

## Accessibility

- Buttons and icon-only controls need accessible labels.
- Tables should preserve readable headers and keyboard-friendly row actions.
- Dialogs and destructive replay actions must have explicit confirmation.
- Live updates should not steal focus while a user is inspecting data.

## Tests Required

For component changes:

- Component behavior tests for filters, selection, pause/resume, replay
  confirmation, and other logic-heavy UI.
- XSS tests or assertions for SQL text and database errors when rendering
  untrusted content.
- Playwright smoke tests for major flows once the app shell exists.
- Screenshots in PRs for visible UI changes.

## Common Mistakes

- Do not build a landing page when the task asks for the SQL Lens app.
- Do not put cards inside cards or make dense operational screens look like
  marketing pages.
- Do not use `dangerouslySetInnerHTML` for SQL or database errors.
- Do not make live WebSocket updates reorder visible tables while the user is
  inspecting paused data.
