# Implement — Issue 074: SQL Detail page

## 1. Update DetailDrawerProvider

- [ ] Add `selectedEventId: string | null` state.
- [ ] Change `openDrawer()` signature to `openDrawer(eventId?: string)`.
- [ ] When eventId is provided, store it; when not, store null.
- [ ] `closeDrawer()` clears selectedEventId.
- [ ] Export `selectedEventId` from context value.

## 2. Create SqlDetail component

- [ ] Create `src/components/sql/sql-detail.tsx`:
      - Props: `{ eventId: string }`.
      - Uses `useSqlEvent(eventId)` for data.
      - Loading: skeleton blocks per section.
      - Error/Not found: AlertTriangle + message.
      - Sections: Summary, SQL, Parameters, Timings, Result, Error,
        Connection, Protocol metadata, Replay.
      - Copy button for SQL text (`navigator.clipboard.writeText` + toast).
      - Toggle original/expanded SQL when they differ.
      - Collapsible metadata JSON block.

## 3. Update detail-drawer.tsx

- [ ] Import `SqlDetail` and `selectedEventId` from context.
- [ ] When `selectedEventId` is set, render `<SqlDetail eventId={...} />`.
- [ ] When null, show placeholder text.
- [ ] Sheet title: "SQL Detail" (or event ID).

## 4. Update sql-events.tsx

- [ ] Change `handleSelectEvent` to call `openDrawer(event.id)`.
- [ ] Remove the `_id` unused parameter warning.

## Validation gates

- [ ] `npm run build` → exit 0.
- [ ] `npm run typecheck` → exit 0.
- [ ] `npm run test` → exit 0.
- [ ] No `fetch` in new/changed files.
- [ ] No hardcoded status colors.

## Rollback

All changes in `crates/sql-lens-app/web/src/`. `git checkout -- src/` reverts.
