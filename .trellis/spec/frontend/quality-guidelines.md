# Frontend Quality Guidelines

> Quality standards for the planned SQL Lens frontend.

## Overview

The `web/` app skeleton exists at `crates/sql-lens-app/web/` (Issue 064). It behaves
like a developer debugging tool: fast to scan, safe with untrusted SQL text, and
stable under live updates. Feature implementations (live data, charts, replay)
land in follow-up issues and must keep these qualities.

## Code Quality

- Use TypeScript and avoid `any`.
- Keep feature code under `web/src/features/*`.
- Keep shared UI, API, formatting, and websocket helpers in their documented
  directories.
- Prefer small, explicit components and hooks over broad generic abstractions.
- Add dependencies only when they are part of the planned stack or justified by
  the task.

## Build & Decoupling Contract (established by Issue 064)

- The frontend must build standalone: `cd crates/sql-lens-app/web && npm run build`
  exits 0 with no backend running.
- The skeleton must contain **no concrete backend coupling**: no `fetch(`,
  `XMLHttpRequest`, or `new WebSocket(...)` calls, and no `/api/v1` literals.
  Verify before finishing a frontend task:
  ```bash
  grep -rnE "fetch\(|XMLHttpRequest|new WebSocket" crates/sql-lens-app/web/src/  # → no matches
  grep -rn "/api/v1" crates/sql-lens-app/web/src/                                  # → no matches
  ```
- The only permitted backend reference is the config-only `apiBaseUrl` reader in
  `src/lib/api/config.ts` (see directory-structure.md "API base URL wiring").
- Typed API client functions (Issue 066) and TanStack Query (Issue 067) are the
  sanctioned homes for runtime calls. Do not scatter `fetch` calls through
  components or route stubs.

## Status Color Contract (reinforced by Issue 065)

- Status semantics use the `--status-ok/slow/error/unknown` tokens surfaced as
  `text-status-*` / `bg-status-*`. Never hardcode `text-red-*`, `text-green-*`,
  `text-amber-*`, `text-yellow-*`, `text-emerald-*`, or `text-rose-*` for
  status. Verify before finishing a frontend task:
  ```bash
  grep -rnE "text-(red|green|amber|yellow|emerald|rose)-[0-9]" crates/sql-lens-app/web/src/  # → none used for status
  ```
- The `--destructive` token is for destructive **actions** (e.g. "Delete"
  buttons), not for query status. It is the only sanctioned red outside the
  `--status-*` family.
- Color is never the only status signal: pair `text-status-*` with an icon or
  word (e.g. a `Badge` with `text-status-error` plus the text "Error").

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
- Frontend type-check and build: `cd crates/sql-lens-app/web && npm run build`
  (runs `tsc -b` strict type-check + `vite build`) must pass.

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
