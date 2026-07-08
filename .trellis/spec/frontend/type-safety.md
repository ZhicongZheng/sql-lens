# Type Safety

> Type safety conventions for the planned SQL Lens frontend.

## Overview

The frontend will use TypeScript. API payloads must stay aligned with backend
response schemas in `crates/sql-lens-api`; SQL text and database errors are
untrusted content even when they originate from local development databases.

## Type Organization

- Shared API-facing types belong under `web/src/types` or `web/src/lib/api`.
- Feature-local view models belong inside `web/src/features/<feature>`.
- Component prop types stay next to the component unless reused.
- Keep backend JSON field names as `snake_case` in API DTO types.
- Convert API DTOs to display view models at feature boundaries when needed.

## API Payloads

- Avoid `any` for REST and WebSocket payloads.
- Prefer generated or shared DTO definitions when schema generation is available.
- Until generation exists, mirror backend response structs deliberately and update
  frontend types in the same task as API response changes.
- Use typed decoders or guards when reading untyped WebSocket events.

## Validation

- Runtime validation is not implemented yet.
- Introduce a validation library only with a task-level decision that explains
  where decoding happens and how errors surface to users.
- At minimum, validate untrusted WebSocket payload shape before rendering or
  mutating query cache.

## Common Patterns

- Use discriminated unions for WebSocket message kinds and UI modes.
- Use branded or narrow string types only when they prevent real mistakes, such
  as event IDs or connection IDs crossing feature boundaries.
- Keep status and protocol values in typed enums or literal unions when they are
  consumed by multiple components.

## Forbidden Patterns

- Do not use `any` for API responses, WebSocket messages, SQL event rows, or
  filter objects.
- Do not cast raw payload fields in multiple places; create a shared decoder,
  type guard, or normalizer before adding another reader.
- Do not render SQL or database error text as HTML.
- Do not invent frontend-only status strings that are not mapped from backend
  contracts.

## Tests Required

For type contract changes:

- Compile/type-check the frontend once it exists.
- Add tests for payload guards and URL filter parsing.
- Add XSS-focused tests for SQL and error rendering surfaces.
