# Issue 076: Build parameter table component

## Goal

Display SQL statement parameters in a structured table within the SQL Detail
view, with clear handling of redacted values, binary summaries, and long
values. This replaces the placeholder "No parameters available" block with
real parameter data from the backend.

## Requirements

### R1 — TypeScript types for parameters

Add parameter types to `src/types/index.ts`:

- `SqlParameterValue` — discriminated union matching the backend enum:
  `Null | Integer | Unsigned | Float | Boolean | String | Date | Time |
  Timestamp | Json | BinarySummary | Unsupported`. Each variant is a tagged
  object `{ type: string, value: string | number | boolean }` (the backend
  serializes the enum as a tagged JSON object).
- `SqlParameter` — `{ index: number, name?: string, value: SqlParameterValue,
  redacted: boolean }`.
- Add `parameters: SqlParameter[]` field to the existing `SqlEvent` type
  (the backend already sends it; the frontend type was missing it).

### R2 — ParameterTable component

Create `src/components/sql/parameter-table.tsx`:

- Props: `{ parameters: SqlParameter[] }`.
- Uses the shadcn `Table` component.
- Columns: Index | Name | Type | Value | Redaction state.
- Empty state: "No parameters" when the array is empty.

### R3 — Redacted value display

- When `redacted === true`: show a `Badge variant="secondary"` with a
  `ShieldCheckIcon` + "Redacted" label. The value cell shows the masked
  value (e.g. `"***"`) in muted text.
- Color: redaction indicator uses `text-status-ok` (green = safe/redacted),
  not a hardcoded color.

### R4 — Value formatting per type

Each `SqlParameterValue` variant is rendered appropriately:

| Variant | Display |
|---|---|
| Null | `NULL` (muted italic) |
| Integer / Unsigned / Float | the number, monospace |
| Boolean | `true` / `false`, monospace |
| String | quoted string, monospace, truncated with expansion (R5) |
| Date / Time / Timestamp | the string, monospace |
| Json | formatted JSON in a `<pre>` block |
| BinarySummary | the summary string (e.g. "32 bytes"), monospace |
| Unsupported | the raw string, muted italic |

Type column shows the variant tag (lowercase, e.g. `string`, `integer`).

### R5 — Long value truncation + expansion

- String/Json values longer than 80 characters are truncated with `…` and
  a "Show more" toggle button.
- Clicking "Show more" expands to full value; clicking "Show less"
  collapses.
- Use a local `expanded` state per row.

### R6 — Binary safety

- All parameter values are rendered as **escaped text**, never as HTML.
- No `dangerouslySetInnerHTML` anywhere.
- JSON values are `JSON.stringify`-ed before display (not parsed and
  re-rendered as HTML).

### R7 — SQL Detail integration

Update `src/components/sql/sql-detail.tsx`:
- Replace the `ParametersBlock` placeholder with `<ParameterTable
  parameters={event.parameters} />`.

## Acceptance Criteria

- [ ] `npm run build` exits 0.
- [ ] `npm run typecheck` exits 0.
- [ ] `npm run test` exits 0.
- [ ] `SqlEvent` type includes a `parameters: SqlParameter[]` field.
- [ ] ParameterTable renders columns: Index, Name, Type, Value, Redaction.
- [ ] Redacted parameters show a "Redacted" badge with `text-status-ok`.
- [ ] Long string/JSON values truncate with a working Show more/less toggle.
- [ ] Binary summary values display safely as text.
- [ ] NULL values display as muted "NULL".
- [ ] No `dangerouslySetInnerHTML` in the component.
- [ ] No `fetch` calls in the component.
- [ ] No hardcoded status colors.

## Out of Scope

- Parameter editing (read-only display only).
- Custom redaction policy UI (backend controls redaction).
- Parameter type icons (text labels suffice).

## Constraints

- Parameter values are untrusted display text — escaped, never HTML.
- Use shadcn `Table`, `Badge`, `Button` from 065 baseline.
- No new dependencies.
