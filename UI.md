# SQL Lens Web UI

## Product Feel

SQL Lens is a developer tool. The UI should be dense, calm, fast, and built for repeated inspection.

It should feel closer to an observability console than a marketing site.

## Stack

- React.
- TypeScript.
- TailwindCSS.
- shadcn/ui.
- TanStack Query.
- Monaco Editor.
- ECharts.

## Information Architecture

Primary navigation:

- Dashboard.
- SQL.
- Connections.
- Statistics.
- Replay.
- Settings.

Secondary views:

- SQL Detail.
- Connection Detail.
- Export.
- Plugin status.

## Layout

Desktop:

- Left sidebar for navigation.
- Top bar for active target, capture status, and global search.
- Main content area with dense tables and charts.
- Right-side detail drawer for quick inspection where useful.

Mobile:

- Top navigation.
- Filter drawer.
- SQL cards instead of wide tables.
- Detail pages instead of side-by-side panes.

## Dashboard

Widgets:

- QPS.
- Latency p50, p95, p99.
- Active connections.
- Slow SQL.
- Error SQL.
- Protocol mix.
- Top slow fingerprints.
- Top error fingerprints.
- Recent error timeline.

Interactions:

- Click a metric to filter SQL List.
- Click a fingerprint to open Statistics detail.
- Time window selector.

## SQL List

Purpose: the main working surface.

Columns:

- Time.
- Protocol.
- Database.
- User.
- Client.
- Duration.
- Status.
- Rows.
- SQL preview.

Controls:

- Text search.
- Protocol filter.
- Status filter.
- Duration range.
- Database filter.
- User filter.
- Pause live updates.
- Clear local filters.

Behavior:

- Live events appear at the top.
- User can pause auto-scroll.
- Slow and error statuses are visually distinct.
- SQL previews are monospace and truncated cleanly.

## SQL Detail

Sections:

- Summary.
- Original SQL.
- Expanded SQL.
- Parameters.
- Timings.
- Result.
- Error.
- Connection.
- Protocol metadata.
- Replay.

Monaco Editor:

- Read-only.
- SQL syntax highlighting.
- Copy button.
- Toggle original and expanded SQL.

Parameter table:

- Index.
- Name when available.
- Type.
- Value.
- Redaction state.

## Connections

Columns:

- Connection ID.
- Protocol.
- Client.
- Backend.
- User.
- Database.
- State.
- Connected time.
- Last activity.
- Query count.
- Bytes in/out.

Interactions:

- Filter by active or closed.
- Open connection detail.
- View SQL events for a connection.

## Statistics

Charts:

- QPS over time.
- Latency percentiles over time.
- Error rate.
- Slow SQL trend.
- Top fingerprints.
- Top databases.
- Top users.

Use ECharts with restrained colors and clear legends.

## Replay

Replay must be careful.

UI requirements:

- Show target connection or configured replay target.
- Show SQL preview.
- Warn on mutating SQL.
- Require explicit confirmation for mutation.
- Show result or error.
- Keep replay history separate from captured traffic unless explicitly enabled.

## Settings

Sections:

- Proxy.
- Backend.
- Storage.
- Redaction.
- Slow SQL threshold.
- Auth.
- Plugins.
- Exporters.

Settings should distinguish runtime-editable fields from fields requiring restart.

## Theme

Requirements:

- Light and dark mode.
- High contrast for status indicators.
- No one-note palette.
- Status colors:
  - OK: green.
  - Slow: amber.
  - Error: red.
  - Unknown: neutral.

## Accessibility

- Keyboard navigable tables and filters.
- Visible focus states.
- Proper labels for icon buttons.
- Color is not the only status indicator.
- SQL text remains readable in dark mode.

## Recommended React Structure

```text
web/
  src/
    app/
      routes/
      providers/
    components/
      ui/
      layout/
      charts/
      sql/
      connections/
    features/
      dashboard/
      sql-events/
      connections/
      statistics/
      replay/
      settings/
    lib/
      api/
      websocket/
      format/
      filters/
    types/
    styles/
```

