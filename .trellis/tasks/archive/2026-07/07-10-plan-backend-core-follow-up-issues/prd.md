# Plan backend core follow-up issues

## Goal

Bring `ISSUES.md` in line with the current backend gap analysis by adding
actionable follow-up issues without rewriting completed issue history.

## Confirmed Facts

- Issues 016, 018, 082, and 089 are archived, but each left a runtime or
  configuration integration gap outside its completed primitive scope.
- Issue 090 now provides plugin hook contracts; exporter and runtime dispatch
  work remains separate.
- Issues 091-093 already cover the three planned exporters, while Issues
  100-102 intentionally cover research rather than protocol implementation.
- The highest existing issue number is 114.

## Requirements

- Add follow-up issues 115-118 for capture-pipeline runtime fan-out, configured
  slow-query classification, configured retention enforcement, and active
  session draining during application shutdown.
- Add standalone issues 119-125 for runtime TLS modes, plugin dispatch and
  loading, DuckDB storage, guarded replay execution, an EXPLAIN helper, MySQL
  COM_QUERY attributes, and additional MySQL prepared-statement parameter types.
- Give each issue a concise description, testable acceptance criteria, labels,
  priority, difficulty, estimate, and explicit dependencies.
- Keep exporter and multi-protocol research issues unchanged; new protocol
  adapter work should follow their research outcomes rather than predetermine
  an implementation design in this backlog update.

## Proposed Issue Map

| Issue | Title | Priority | Dependencies |
| --- | --- | --- | --- |
| 115 | Wire capture pipeline into app runtime fan-out | P0 | 018, 109, 112 |
| 116 | Apply configured slow-query threshold at runtime | P0 | 082, 109 |
| 117 | Enforce configured retention in app runtime | P1 | 089, 112 |
| 118 | Drain active proxy sessions during app shutdown | P1 | 016, 109 |
| 119 | Implement configured TLS modes in proxy runtime | P1 | 007, 014 |
| 120 | Add plugin runtime dispatch and loading | P2 | 090, 115 |
| 121 | Add DuckDB storage backend | P3 | 086, 087, 088 |
| 122 | Add guarded replay execution API | P2 | 080 |
| 123 | Add EXPLAIN helper API | P2 | 080 |
| 124 | Capture MySQL COM_QUERY attributes | P2 | 043 |
| 125 | Decode additional MySQL prepared-statement parameter types | P3 | 050 |

## Acceptance Criteria

- [x] `ISSUES.md` contains exactly the proposed Issues 115-125 in sequence.
- [x] Every new issue has a description, acceptance criteria, labels, priority,
      difficulty, estimate, and dependency list.
- [x] Follow-up issues reference the completed primitive issue they extend.
- [x] No completed issue text, issue number, exporter issue, or protocol research
      issue is modified.
- [x] Markdown formatting remains valid and no duplicate issue number is added.

## Out Of Scope

- Implementing any listed backend feature.
- Changing completed issue records or task archives.
- Creating PostgreSQL, SQLite execution-surface, or ClickHouse adapter issues
  before their corresponding research issues establish a recommended path.
