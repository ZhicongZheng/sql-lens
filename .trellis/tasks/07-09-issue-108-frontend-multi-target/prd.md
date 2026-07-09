# Issue 108: Frontend multi-target proxy support

## Goal

Surface `target_name` throughout the frontend so users running multiple
proxy targets (e.g. `mysql-local` + `starrocks-local`) can see which target
captured each event and filter by target. The `target_name` field already
exists in all API responses and TypeScript types — this task adds UI
visibility and filtering.

## Requirements

### R1 — SQL List: target column

Add a `Target` column to the SQL event table (between Time and Protocol):

| Position | Column | Width |
|---|---|---|
| 1 | Time | w-[72px] |
| **2** | **Target** | **w-[100px]** |
| 3 | Protocol | w-[80px] |
| ... | (rest unchanged) | |

Display `event.target_name` in a monospace text cell. When empty, show "—".

### R2 — SQL Detail: target in Summary

Add target_name to the Summary section of `SqlDetail` (it's already shown in
the Connection section, but add it to Summary for quick visibility alongside
protocol/database/user).

### R3 — Filter bar: target filter

Add a `target_name` filter control to the SQL List filter bar:

- Shadcn `Input` with placeholder "Target" and `aria-label="Target"`.
- URL param: `target_name`.
- API param: `target_name` (already in `SqlEventQueryParams`).
- Positioned after the Protocol filter.

### R4 — Topbar target indicator (dynamic)

Update the topbar target badge to show the first target from the API when
available, falling back to the current hardcoded `mysql-local`. Use
`useProtocols()` (returns available protocols + databases; target names come
from the statistics or a future endpoint) or keep the placeholder until a
dedicated target-list endpoint exists.

Decision: keep the topbar badge as `mysql-local` placeholder for now. A
follow-up issue can wire it to a real target-list API. This avoids
introducing speculative fetch calls.

## Acceptance Criteria

- [ ] `npm run build` exits 0.
- [ ] `npm run typecheck` exits 0.
- [ ] `npm run test` exits 0.
- [ ] SQL List table has a Target column showing `event.target_name`.
- [ ] SQL Detail Summary section shows target name.
- [ ] Filter bar has a `target_name` input that updates URL params and API
      query.
- [ ] Target filter appears in the active filter count.
- [ ] No `fetch` calls in changed files.
- [ ] No hardcoded status colors.
- [ ] Dark mode renders correctly.

## Out of Scope

- Dynamic topbar target indicator from API (placeholder kept).
- Target selector dropdown (needs a target-list API endpoint).
- Multi-target connection views (future issue).

## Constraints

- `target_name` already in types — no type changes needed.
- No new dependencies.
