# Implementation Plan: Add XSS regression tests (Issue 103)

## Overview
Ordered checklist for implementing XSS regression tests, with validation commands and review gates.

## Prerequisites
- [ ] Verify test dependencies are installed:
  ```bash
  cd crates/sql-lens-app/web
  npm ls vitest @testing-library/react @testing-library/user-event jsdom
  ```
- [ ] Check if existing tests exist (to understand patterns):
  ```bash
  find crates/sql-lens-app/web/src -name "*.test.ts*" -o -name "*.test.tsx" | head -10
  ```
- [ ] Review `prd.md` and `design.md` for alignment

## Phase 1: Test Infrastructure Setup

### 1.1 Verify vitest configuration
Read `vitest.config.ts` to confirm:
- jsdom environment is configured
- React Testing Library setup exists
- Test file patterns include `**/__tests__/**/*.{test,spec}.{ts,tsx}`

**Validation**:
```bash
cat crates/sql-lens-app/web/vitest.config.ts
```

### 1.2 Create test directory structure
```bash
mkdir -p crates/sql-lens-app/web/src/app/routes/__tests__
mkdir -p crates/sql-lens-app/web/src/features/sql-events/__tests__
```

## Phase 2: SQL List XSS Tests

### 2.1 Create `sql-events.test.tsx`
File: `src/app/routes/__tests__/sql-events.test.tsx`

Structure:
```typescript
import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { SqlEventsRoute } from '../sql-events';

// Mock useSqlEvents hook
vi.mock('@/lib/api/hooks/use-sql-events', () => ({
  useSqlEvents: vi.fn(),
}));

describe('SQL List XSS prevention', () => {
  const xssPayloads = [
    "<script>alert('xss')</script>",
    "<img src=x onerror=alert('xss')>",
    // ... more payloads
  ];

  xssPayloads.forEach((payload, index) => {
    it(`escapes payload ${index + 1} in SQL preview`, () => {
      // Mock hook to return event with payload as original_sql
      // Render SqlEventsRoute
      // Assert: payload appears as text, no script elements
    });
  });
});
```

### 2.2 Implement mock for `useSqlEvents`
- Mock should return `PaginatedResponse<SqlEvent>` with controlled malicious events
- Use `vi.mocked()` or direct mock implementation

**Validation**:
```bash
npm run typecheck
# Expected: No type errors in test file
```

## Phase 3: SQL Detail XSS Tests

### 3.1 Create `SqlDetailPage.test.tsx`
File: `src/features/sql-events/__tests__/SqlDetailPage.test.tsx`

Structure:
```typescript
import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { SqlDetailPage } from '../SqlDetailPage';

// Mock useSqlEvent hook
vi.mock('../hooks/useSqlEvent', () => ({
  useSqlEvent: vi.fn(),
}));

describe('SQL Detail XSS prevention', () => {
  // Test cases for:
  // - original_sql / expanded_sql in SqlMonacoViewer
  // - error messages in SqlError
  // - parameter values in SqlParameterTable
  // - metadata in SqlSummary
});
```

### 3.2 XSS payloads for SQL Detail contexts
- **SQL fields** (original_sql, expanded_sql): Same as SQL List
- **Error messages**: `<script>alert('error xss')</script>`
- **Parameter values**: `<img src=x onerror=alert('param xss')>`
- **Metadata fields** (user, database, addresses): `<svg onload=alert('meta xss')>`

### 3.3 Implement mock for `useSqlEvent`
- Mock returns `SqlEvent` with malicious data in target fields
- Use `vi.fn().mockReturnValue({ data: maliciousEvent, isLoading: false })`

**Validation**:
```bash
npm run typecheck
```

## Phase 4: Test Execution & Validation

### 4.1 Run tests
```bash
cd crates/sql-lens-app/web
npm test -- --run src/app/routes/__tests__/sql-events.test.tsx src/features/sql-events/__tests__/SqlDetailPage.test.tsx
```

### 4.2 Verify test output
- All tests should pass
- Review test output to confirm payloads are rendered as text (not executed)
- If any test fails, debug: check if component is actually escaping or if assertion is wrong

**Validation**:
```bash
npm test -- --run 2>&1 | tail -30
```

## Phase 5: Documentation & Polish

### 5.1 Add test comments
Ensure each `it()` block has a comment documenting:
- The payload
- The attack vector
- The expected safe behavior

Example:
```typescript
it('escapes <script> tag in SQL preview', () => {
  // Payload: <script>alert('xss')</script>
  // Attack vector: Direct script injection via SQL text column
  // Expected: Text content displays literally, no script execution
});
```

### 5.2 Optional: Add `SqlMonacoViewer.test.tsx`
If time permits, add isolated tests for Monaco viewer:
- Verify Monaco treats `<script>` as SQL text, not HTML
- Verify no DOM injection from value prop

**Validation**:
```bash
npm run typecheck
npm test -- --run src/features/sql-events/__tests__/
```

## Phase 6: Integration Verification

### 6.1 Full test suite run
```bash
npm test -- --run
```

### 6.2 Confirm no regressions
- Existing tests (if any) should still pass
- New XSS tests should not introduce flakiness

**Validation**:
```bash
npm test -- --run 2>&1 | grep -E "(PASS|FAIL|Tests:)"
```

## Rollback Plan
If tests cause issues:
1. Delete test files
2. No production code changes to revert (tests are additive)
3. CI will not be affected until tests are added to pipeline (Issue 095)

## Estimated Effort
- Per Issue 103: 5 hours
- Breakdown:
  - Test infrastructure & mocks: 1 hour
  - SQL List XSS tests: 1.5 hours
  - SQL Detail XSS tests: 1.5 hours
  - Validation & documentation: 1 hour
