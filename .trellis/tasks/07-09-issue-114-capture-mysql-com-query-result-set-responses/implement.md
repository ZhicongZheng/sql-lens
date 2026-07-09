# Issue 114 Implementation Plan

## Checklist

- [x] Inspect current MySQL command parser and backend query finalization tests.
- [x] Fix `COM_QUERY` SQL extraction if it retains command or packet prefix bytes.
- [x] Add a result-set response tracker to MySQL connection state.
- [x] Finalize pending `COM_QUERY` on result-set terminal EOF/OK packets.
- [x] Populate returned row count for result-set query events.
- [x] Add protocol unit tests for clean SQL, OK/ERR regression, result-set
      capture, and row counting.
- [x] Add Docker-only/env-gated app smoke coverage for proxied `SELECT`.
- [x] Update `ISSUES.md` with Issue 114.
- [x] Run validation:
  - `rtk cargo fmt --check`
  - `rtk cargo test -p sql-lens-protocol-mysql`
  - `rtk cargo test -p sql-lens-app`
  - `rtk cargo test --workspace`
  - `rtk cargo clippy --workspace --all-targets -- -D warnings`
- [ ] Archive task, record journal, commit, and push.

## Risk Points

- MySQL packet sequencing differs between EOF terminators and OK terminators
  depending on server capabilities.
- The current observer is byte-slice based, not a full stream reassembler, so
  this task should keep scope to complete packets already supported by existing
  parser assumptions.
- Do not log packet payloads, SQL parameters, result rows, passwords, or
  authentication data while debugging.
