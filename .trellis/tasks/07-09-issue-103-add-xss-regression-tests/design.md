# Design: Add XSS regression tests (Issue 103)

## Overview
This document describes the technical design for XSS regression tests, including test structure, payload selection, rendering verification strategy, and integration with existing test infrastructure.

## Architecture

### Test Structure
```
src/
  app/
    routes/
      __tests__/
        sql-events.test.tsx          # SQL List XSS tests
  features/
    sql-events/
      __tests__/
        SqlDetailPage.test.tsx       # SQL Detail XSS tests
        SqlMonacoViewer.test.tsx     # Monaco viewer XSS tests (optional)
```

### Test Framework
- **Runner**: vitest (configured in `vitest.config.ts`)
- **DOM**: jsdom (implicit via vitest config or explicit in test setup)
- **Assertions**: `@testing-library/react` render + `screen` queries
- **User events**: `@testing-library/user-event` (if interaction needed, likely not for XSS)

### XSS Payload Categories
Tests will use a curated set of payloads targeting different attack surfaces:

1. **Script injection**:
   - `<script>alert('xss')</script>`
   - `<script>document.body.innerHTML='pwned'</script>`

2. **Event handler injection**:
   - `<img src=x onerror=alert('xss')>`
   - `<svg onload=alert('xss')>`

3. **javascript: URL injection**:
   - `<a href="javascript:alert('xss')">click</a>`

4. **HTML entity evasion**:
   - `&lt;script&gt;alert('xss')&lt;/script&gt;` (should render as text, not execute)

5. **Attribute injection** (if applicable to parameter display):
   - `"><script>alert('xss')</script>`

### Test Strategy

#### SQL List Tests (`sql-events.test.tsx`)
Target rendering contexts:
- `EventRow` component: `original_sql` preview in table cell
- Status badge: `status` field (should be safe, but test anyway)
- Metadata fields: `target_name`, `protocol`, `database`, `user`, `client_addr`, `backend_addr`

Test approach:
1. Render `SqlEventsRoute` (or `EventRow` directly) with mock event containing malicious `original_sql`
2. Query the rendered cell containing SQL preview
3. Assert: malicious content is present as **text content**, not parsed as DOM elements
4. Assert: no `<script>` elements exist in document
5. Assert: no error handlers are registered (indirect via absence of side effects)

#### SQL Detail Tests (`SqlDetailPage.test.tsx`)
Target rendering contexts:
- `SqlMonacoViewer`: `original_sql` and `expanded_sql` (Monaco Editor content)
- `SqlSummary`: metadata fields (timestamp, protocol, addresses, etc.)
- `SqlError`: error messages from `metadata.mysql.error.message`
- `SqlParameterTable`: parameter `value` fields

Test approach:
1. Mock `useSqlEvent` hook to return event with malicious payloads in various fields
2. Render `SqlDetailPage`
3. For each rendering context:
   - Assert: malicious content appears as text/innerText, not as child elements
   - Assert: no injected DOM nodes (e.g., `querySelector('script')` returns null)
   - Assert: no console errors from script execution (if detectable)

#### Monaco Viewer Specific Tests (optional, `SqlMonacoViewer.test.tsx`)
- Monaco Editor renders content in a controlled `<textarea>` or canvas, not raw HTML
- Test that even if `value` prop contains `<script>`, it is treated as SQL text
- Verify: `document.querySelectorAll('script')` count unchanged after render

### Mock Strategy
- Use vitest's `vi.mock()` to mock API client or React Query hooks
- Mock `useSqlEvents` to return controlled malicious events for SQL List
- Mock `useSqlEvent` to return controlled malicious event for SQL Detail
- Avoid real network calls; all data is synthetic

### Assertion Patterns
Safe rendering verification:
```typescript
// After rendering with malicious payload
const sqlCell = screen.getByText(/<script>/); // Should find text content
expect(sqlCell).toBeInTheDocument();

// Verify no script execution
expect(document.querySelector('script:not([type="application/json"])')).toBeNull();

// Verify HTML is escaped (text appears literally)
expect(sqlCell.textContent).toContain('<script>alert');
```

### Test Isolation
- Each test renders fresh component tree
- No shared state between tests
- Cleanup via `cleanup()` from `@testing-library/react` (auto via vitest config)

## File Locations
- SQL List tests: `src/app/routes/__tests__/sql-events.test.tsx`
- SQL Detail tests: `src/features/sql-events/__tests__/SqlDetailPage.test.tsx`
- Test utilities (if needed): `src/test-utils.tsx` or inline in test files

## Payload Documentation
Each test case should include a comment:
```typescript
it('escapes <script> tag in SQL preview', () => {
  // Payload: <script>alert('xss')</script>
  // Attack vector: Direct script injection via SQL text
  // ...
});
```

## Verification
- Run `npm test` to execute all new tests
- All tests must pass without false positives (i.e., tests should fail if XSS protection is removed)
- Manual review: inspect test output to confirm payloads are treated as text

## Limitations
- These are **regression tests**, not a comprehensive security audit
- Focus on UI rendering layer; backend redaction (Issue 056) is out of scope
- Monaco Editor's internal security is assumed (third-party library); tests verify our usage is safe
