# Issue 080: Add replay preview API

## Goal

Add a backend-only replay preview endpoint that lets users inspect the SQL that
would be replayed and see a mutation warning without opening any database
connection or executing SQL.

The product value is a safe first step toward replay: users can inspect captured
or pasted SQL and understand whether it looks read-only or mutating before a
future execute endpoint exists.

## Source Issue

Issue 080: Add replay preview API.

- Description: Implement replay preview endpoint that renders SQL and risk
  classification without executing it.
- Acceptance: endpoint accepts event ID or SQL payload; response includes final
  SQL and mutation warning; no SQL is executed.
- Labels: `area:api`, `area:replay`, `type:feature`
- Priority: P1
- Dependencies: Issue 029, Issue 055

## Requirements

- R1. Add `POST /api/v1/replay/preview`.
- R2. Accept either a captured SQL event ID or a raw SQL payload.
- R3. For event ID input, load the event from the existing in-memory event
  store and choose replay SQL from `expanded_sql` when present, otherwise
  `original_sql`.
- R4. For raw SQL input, use the submitted SQL as the preview SQL.
- R5. Return a protocol-neutral replay preview response containing final SQL,
  mutation classification, and a warning flag/message for mutating SQL.
- R6. Include enough source context for clients to distinguish captured-event
  previews from raw-SQL previews.
- R7. Do not execute SQL, dial a backend, enqueue replay work, or mutate stored
  events.
- R8. Use the existing API error envelope for invalid input and missing events.
- R9. Keep replay execute API, frontend UI, authentication, and database
  connection logic out of scope.

## Acceptance Criteria

- [x] `POST /api/v1/replay/preview` is registered under the API router.
- [x] Event ID preview returns SQL from `expanded_sql` when present.
- [x] Event ID preview falls back to `original_sql` when `expanded_sql` is absent.
- [x] Raw SQL preview returns the submitted SQL.
- [x] Mutating SQL is flagged with a mutation warning.
- [x] Read-only SQL is not flagged as mutating.
- [x] Empty or ambiguous preview requests return `BAD_REQUEST`.
- [x] Missing event IDs return `NOT_FOUND`.
- [x] Tests prove preview does not modify event storage.
- [x] `rtk cargo fmt --check` passes.
- [x] `rtk cargo test --workspace` passes.
- [x] `rtk cargo clippy --workspace --all-targets -- -D warnings` passes.

## Notes

- `API.md` already names `ReplayPreview` and `ReplayRequest` as core schema
  names, but the detailed wire shape still needs to be added during
  implementation.
- The existing config has `replay.enabled` and
  `require_confirmation_for_mutations`; preview should remain non-executing even
  when replay execute is disabled or absent.

## Implementation Status

Implemented preview-only backend API. The endpoint supports event and raw SQL
sources, uses expanded SQL when available, conservatively flags mutations, and
does not modify stored events.
