# Enforce configured retention in app runtime

## Goal

Enforce configured data retention policies at the application runtime layer to ensure data is automatically purged according to user-configured retention settings, complementing any database-level retention mechanisms.

## Background

This task depends on:
- Issue 089: (retention configuration mechanism)
- Issue 112: (retention configuration storage/definition)

The application needs to actively enforce retention policies at runtime, not just define and store them.

## Requirements

- Application runtime must periodically scan and delete data exceeding configured retention periods
- Retention enforcement must respect per-table or per-query retention settings
- Enforcement must be configurable (enable/disable, interval)
- Must handle concurrent capture operations without data races
- Must log enforcement actions for auditability
- Must not block capture pipeline during enforcement runs

## Acceptance Criteria

- [ ] Retention enforcement job runs on configurable schedule
- [ ] Data older than configured retention period is deleted from storage
- [ ] Enforcement respects table/query-specific retention overrides
- [ ] Concurrent capture operations continue without blocking during enforcement
- [ ] Enforcement actions are logged with before/after counts
- [ ] Configuration changes to retention settings take effect on next enforcement cycle
- [ ] Graceful handling when retention enforcement fails (does not crash app)

## Notes

- Complex backend storage task requiring planning artifacts (prd.md, design.md, implement.md) — all created.
- Dependencies: Issue 089 (retention config mechanism), Issue 112 (retention config storage)
- Labels: area:backend, area:storage, area:config, type:feature
- Priority: P1, Difficulty: Hard, Estimated: 8h
- Ready for Phase 1.1 planning iteration.
