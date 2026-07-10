# Task: Add retention policy enforcement

**Issue:** 089
**Priority:** P2
**Difficulty:** Medium
**Estimated:** 6h
**Dependencies:** Issue 021, Issue 087
**Labels:** area:storage, type:feature

## Description

Enforce max age, max events, and max bytes retention policies where supported.

## Acceptance Criteria

- Ring buffer respects max events.
- SQLite supports age and event-count cleanup.
- Tests cover cleanup behavior.

## Notes

This is a storage feature task. Full AC available in ISSUES.md Issue 089.
