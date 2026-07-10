# Add XSS regression tests (Issue 103)

## Goal
Implement automated regression tests that verify SQL Lens frontend safely renders SQL text and database error messages without executing malicious scripts, preventing XSS vulnerabilities in the UI.

## Requirements
- Add regression tests covering XSS attack vectors in SQL rendering contexts:
  - SQL List page: SQL preview column, status badges, metadata display
  - SQL Detail page: Original SQL, Expanded SQL (Monaco Editor), error messages, parameter values
- Test that malicious payloads are not executed as HTML/JavaScript:
  - `<script>` tags
  - `onclick` / `onerror` / other event handlers
  - `javascript:` URLs
  - `<img src=x onerror=...>` style payloads
  - HTML entity encoded attacks (`&lt;script&gt;`)
- Use existing test framework (vitest per package.json)
- Follow React Testing Library patterns for component rendering
- Tests should be deterministic and not rely on external services

## Acceptance Criteria
- [ ] Test file exists at `src/app/routes/__tests__/sql-events.test.tsx` or similar
- [ ] Test file exists at `src/features/sql-events/__tests__/SqlDetailPage.test.tsx` or similar
- [ ] SQL List tests verify malicious SQL text is escaped/not executed
- [ ] SQL Detail tests verify malicious SQL (original/expanded) is escaped/not executed
- [ ] SQL Detail tests verify malicious error messages are escaped/not executed
- [ ] SQL Detail tests verify malicious parameter values are escaped/not executed
- [ ] All tests pass with `npm test`
- [ ] Tests cover at least 5 distinct XSS payloads per rendering context
- [ ] Test comments document each payload's attack vector

## Dependencies
- Issue 070: Build SQL List page (required, provides component to test)
- Issue 074: Build SQL Detail page (required, provides component to test)
- Existing test setup (vitest, React Testing Library)

## Constraints
- Must not modify production code unless necessary for testability
- Must not introduce new dependencies without justification
- PRD-only is insufficient; design.md and implement.md required before `task.py start` due to security-sensitive nature and multi-page test coverage
