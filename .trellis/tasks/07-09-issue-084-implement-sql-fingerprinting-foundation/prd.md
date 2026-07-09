# Issue 084: Implement SQL fingerprinting foundation

## Goal

Add a simple protocol-neutral SQL fingerprint function so SQL Lens can group similar queries without depending on a database-specific parser.

## Background

- `ISSUES.md` Issue 084 requires literal normalization, basic whitespace normalization, and tests for common `SELECT`, `INSERT`, `UPDATE`, and `DELETE` statements.
- `SqlEvent` already has `normalized_sql` and `fingerprint` fields in `sql-lens-core`.
- Storage and API already expose fingerprint-aware filtering/response fields, so this task should focus on producing stable fingerprint values at event creation time.
- Issue 083 appears already covered by existing error status, statistics, and API filter behavior, so this is the next backend implementation task.

## Requirements

- Provide a deterministic, protocol-neutral SQL fingerprint function in `sql-lens-core`.
- Normalize literal values to placeholders:
  - single-quoted strings
  - double-quoted strings
  - integer and decimal numeric literals
  - hexadecimal numeric literals
  - basic `NULL`, `TRUE`, and `FALSE` keywords
- Normalize whitespace so equivalent spacing and line breaks produce the same fingerprint.
- Preserve SQL operators, punctuation, identifiers, and statement structure well enough for grouping.
- Wire the function into backend event creation paths so captured query/prepared statement events receive `fingerprint: Some(...)` when SQL text is available.
- Keep behavior conservative and parser-light; malformed SQL should still produce a best-effort fingerprint instead of an error.
- Keep fingerprinting independent from redaction, storage filtering, replay preview, and frontend UI.

## Acceptance Criteria

- [x] Literal values are normalized to placeholders.
- [x] Basic whitespace normalization exists.
- [x] Tests cover common `SELECT`, `INSERT`, `UPDATE`, and `DELETE` statements.
- [x] MySQL adapter event creation populates `SqlEvent.fingerprint` for captured SQL events.
- [x] Existing storage/API fingerprint filters continue to work with generated fingerprints.
- [x] No SQL is executed and no database connection behavior changes.

## Notes

- Out of scope: full SQL parsing, dialect-specific AST normalization, comment stripping beyond simple whitespace handling, frontend UI, export behavior, replay execution behavior, and persistent SQLite schema work.
