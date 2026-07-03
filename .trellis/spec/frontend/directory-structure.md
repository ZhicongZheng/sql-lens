# Frontend Directory Structure

> Frontend code organization for SQL Lens.

## Overview

The SQL Lens frontend is a React and TypeScript developer tool UI. It should be dense, calm, and optimized for inspecting live SQL traffic.

## Directory Layout

```text
web/
└── src/
    ├── app/
    │   ├── routes/
    │   └── providers/
    ├── components/
    │   ├── ui/
    │   ├── layout/
    │   ├── charts/
    │   ├── sql/
    │   └── connections/
    ├── features/
    │   ├── dashboard/
    │   ├── sql-events/
    │   ├── connections/
    │   ├── statistics/
    │   ├── replay/
    │   └── settings/
    ├── lib/
    │   ├── api/
    │   ├── websocket/
    │   ├── format/
    │   └── filters/
    ├── types/
    └── styles/
```

## Module Organization

- `app`: routing, root providers, and app shell wiring.
- `components/ui`: shadcn/ui primitives and thin wrappers.
- `components/layout`: navigation, top bar, split panes, and page frames.
- `components/charts`: ECharts wrappers.
- `components/sql`: SQL display, parameter tables, status badges, and SQL-specific shared UI.
- `features/*`: route-level product features and feature-local components.
- `lib/api`: typed REST API client.
- `lib/websocket`: WebSocket client and subscription helpers.
- `lib/format`: duration, timestamp, SQL preview, and byte formatting.
- `types`: shared frontend types generated from or aligned with API schemas.

## State Rules

- TanStack Query owns server state.
- URL state owns durable filters.
- Component state owns temporary UI state.
- WebSocket events update query cache through explicit handlers.

## Naming Conventions

- React components use `PascalCase`.
- Hooks start with `use`.
- Feature folders use kebab-case.
- API JSON fields stay `snake_case`.
- Local TypeScript variables and properties use normal TypeScript conventions unless mirroring API payloads.

## Common Mistakes

- Do not render SQL text or database error messages as HTML.
- Do not use `any` for API payloads.
- Do not let live WebSocket updates make tables jump while the user is inspecting paused data.
- Do not put feature-specific components into global shared folders unless a second feature needs them.

