# Design — Issue 074: SQL Detail page

## Architecture

### DetailDrawerProvider update

`openDrawer(eventId?)` now accepts an optional event ID. The provider stores
`selectedEventId: string | null`. The `DetailDrawer` component reads this and
conditionally renders `<SqlDetail />` when set.

```
openDrawer("evt_123")  → sets selectedEventId="evt_123", isOpen=true
openDrawer()           → sets selectedEventId=null, isOpen=true (placeholder)
closeDrawer()          → sets selectedEventId=null, isOpen=false
```

### SqlDetail component

Lives in `src/components/sql/sql-detail.tsx` (matches the `components/sql`
directory from UI.md structure). Receives `eventId: string` as a prop.

Uses `useSqlEvent(eventId)` for data. Handles loading/error/not-found states
internally.

Sections are rendered as a vertical stack of labeled groups. Each group:
```
<div className="space-y-1.5">
  <h3 className="text-xs font-medium text-muted-foreground uppercase tracking-wide">Section</h3>
  {content}
</div>
<Separator />
```

### SQL text display

`<pre className="overflow-x-auto rounded-md bg-muted p-3 text-xs font-mono">`

Copy button positioned top-right of the `<pre>` block via a relative/absolute
wrapper.

Toggle between original and expanded SQL uses a small `Button variant=ghost`
that flips a local `showExpanded` boolean. The toggle only appears when
`expanded_sql !== original_sql`.

### Clipboard

`navigator.clipboard.writeText(text)` with `toast("SQL copied")` on success.
Fallback: no toast, just the button does nothing (clipboard API requires
HTTPS or localhost — local dev is fine).

### Metadata display

Protocol metadata (`event.metadata`) is a `Record<string, Record<string,
unknown>>`. Display as a collapsible JSON block. Use a `Button variant=ghost`
to toggle `showMetadata` state. When expanded, render
`JSON.stringify(metadata, null, 2)` in a `<pre>` block.

### Parameters

Parameters come from `metadata` — protocol-specific. For MySQL, they'd be
in `metadata.mysql.parameters` or similar. For now, if no parameter data is
found in metadata, show "No parameters available" placeholder.

## File changes

| File | Change |
|---|---|
| `src/app/providers/detail-drawer-provider.tsx` | update: `selectedEventId`, `openDrawer(id?)` |
| `src/components/layout/detail-drawer.tsx` | update: render `<SqlDetail />` when event selected |
| `src/components/sql/sql-detail.tsx` | **new** — full event detail view |
| `src/app/routes/sql-events.tsx` | update: pass event ID to `openDrawer(event.id)` |

## Non-goals

- No Monaco Editor. No replay execution. No parameter redaction.
