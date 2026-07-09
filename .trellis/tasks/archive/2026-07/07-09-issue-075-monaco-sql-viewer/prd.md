# Issue 075: Integrate Monaco SQL viewer

## Goal

Replace the plain `<pre>` SQL text blocks in the SQL Detail view with a
read-only Monaco Editor instance that provides SQL syntax highlighting,
proper line numbers, and theme-aware rendering. This makes SQL inspection
significantly more readable for complex queries.

## Requirements

### R1 — Monaco Editor installation

- Install `@monaco-editor/react` (the React wrapper) and `monaco-editor`
  (the core editor).
- Configure Vite to bundle Monaco workers locally (not CDN):
  - Add `optimizeDeps: { include: ['monaco-editor'] }` to `vite.config.ts`.
  - Create `src/lib/monaco-config.ts` that imports the editor worker via
    Vite's `?worker` suffix and sets `self.MonacoEnvironment`.
  - Only the base editor worker is needed (SQL is built-in, no extra
    language workers).
- Use `React.lazy` to code-split the Monaco wrapper so it doesn't bloat
  the initial bundle (~5MB). The `SqlDetail` component dynamically imports
  the Monaco wrapper.

### R2 — SqlEditor wrapper component

Create `src/components/sql/sql-editor.tsx` exporting a `SqlEditor` component:

Props:
- `value: string` — the SQL text to display.
- `language?: string` — defaults to `"sql"`.
- `height?: string` — defaults to auto-calculated based on line count
  (capped at `400px`).

Behavior:
- **Read-only**: `options={{ readOnly: true, domReadOnly: true }}`.
- **No minimap**: `minimap: { enabled: false }`.
- **Word wrap**: `wordWrap: "on"`.
- **Line numbers**: `"on"` but compact (`lineNumbersMinChars: 3`).
- **Theme**: follow app theme — use `useTheme()` to detect dark/light,
  map to Monaco's `"vs-dark"` / `"vs"` built-in themes.
- **No scroll beyond last line**: `scrollBeyondLastLine: false`.
- **Automatic layout**: `automaticLayout: true` (resizes with container).
- **No cursor blinking on read-only**: `cursorBlinking: "solid"`.
- **Font**: `fontFamily: "ui-monospace, monospace"` (matches the app's
  monospace style).

### R3 — Replace `<pre>` in SqlDetail

Update `src/components/sql/sql-detail.tsx`:
- Replace the `<SqlBlock>` component (the `<pre>` wrapper) with
  `<SqlEditor>`.
- Keep the Copy button — position it above the editor (not overlaying).
- The toggle between original/expanded SQL still works — it just swaps
  the `value` prop of `SqlEditor`.
- Use `React.Suspense` with a skeleton fallback around the lazy-loaded
  `SqlEditor`.

### R4 — Theme sync

When the app theme toggles (light ↔ dark), the Monaco editor theme
switches between `"vs"` (light) and `"vs-dark"` (dark) without a full
re-mount. Use `useMonaco()` hook + `monaco.editor.setTheme()` in a
`useEffect` keyed to the theme value.

## Acceptance Criteria

- [ ] `npm run build` exits 0.
- [ ] `npm run typecheck` exits 0.
- [ ] `npm run test` exits 0.
- [ ] Monaco Editor renders SQL with syntax highlighting in the SQL Detail
      drawer.
- [ ] Editor is read-only (no cursor editing).
- [ ] Theme follows light/dark mode toggle.
- [ ] Monaco is code-split (lazy loaded, not in the main bundle).
- [ ] Copy button still works.
- [ ] Original/expanded SQL toggle still works.
- [ ] No `fetch` calls in new files.
- [ ] Dark mode renders correctly.

## Out of Scope

- Monaco Editor in other locations (only SQL Detail for now).
- Custom SQL language definition (Monaco's built-in SQL is sufficient).
- Minimap, autocomplete, or other editing features (read-only viewer).

## Constraints

- Monaco is lazy-loaded via `React.lazy` to avoid bloating the initial
  bundle.
- Only the base editor worker is bundled (no JSON/CSS/HTML/TS workers).
- The `<pre>` fallback is removed — Monaco fully replaces it.
