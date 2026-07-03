# SQL Lens Documentation Implementation Plan

## Preconditions

- Task is reviewed and approved to start.
- `task.py start` has been run after approval.
- Work remains documentation-only.

## Files To Create Or Replace

Create root-level documentation files:

- `README.md`
- `ARCHITECTURE.md`
- `PRD.md`
- `PROTOCOL.md`
- `API.md`
- `CONFIG.md`
- `STORAGE.md`
- `SECURITY.md`
- `PLUGIN.md`
- `ROADMAP.md`
- `CONTRIBUTING.md`
- `AGENTS.md`
- `MILESTONE.md`
- `ISSUES.md`
- `TESTING.md`
- `BENCHMARK.md`
- `UI.md`

Existing files:

- Preserve `SQL_Lens_PRD.md` as the original seed unless the user explicitly asks to remove or rewrite it.
- Replace root `AGENTS.md` with the detailed AI coding agent guide requested by the user, while preserving any Trellis block if still present.

## Execution Checklist

1. Confirm current task status and repository files.
2. Generate documentation in coherent groups:
   - Project entry and product docs: `README.md`, `PRD.md`, `ROADMAP.md`.
   - Technical design docs: `ARCHITECTURE.md`, `PROTOCOL.md`, `API.md`, `CONFIG.md`, `STORAGE.md`, `SECURITY.md`, `PLUGIN.md`.
   - Delivery docs: `MILESTONE.md`, `ISSUES.md`, `TESTING.md`, `BENCHMARK.md`.
   - Contributor and AI docs: `CONTRIBUTING.md`, `AGENTS.md`.
   - Product UI docs: `UI.md`.
3. Keep terminology consistent:
   - Product name: SQL Lens.
   - Backend: Rust-first.
   - Product boundary: multi-protocol SQL debug proxy.
   - Initial protocol family: MySQL-compatible.
   - Initial databases: MySQL, StarRocks, TiDB, Doris.
   - Future protocols: PostgreSQL, SQLite integration if feasible, ClickHouse.
4. Ensure `ISSUES.md` has at least 100 issue drafts.
5. Preserve the Trellis-managed block in `AGENTS.md` if it exists.
6. Run validation commands.
7. Summarize files created and validation results.

## Validation Commands

```bash
rtk rg --files
rtk rg -n "MySQL Lens" README.md ARCHITECTURE.md PRD.md PROTOCOL.md API.md CONFIG.md STORAGE.md SECURITY.md PLUGIN.md ROADMAP.md CONTRIBUTING.md AGENTS.md MILESTONE.md ISSUES.md TESTING.md BENCHMARK.md UI.md
rtk rg -n "^## Issue " ISSUES.md
rtk python3 -c "from pathlib import Path; files=['README.md','ARCHITECTURE.md','PRD.md','PROTOCOL.md','API.md','CONFIG.md','STORAGE.md','SECURITY.md','PLUGIN.md','ROADMAP.md','CONTRIBUTING.md','AGENTS.md','MILESTONE.md','ISSUES.md','TESTING.md','BENCHMARK.md','UI.md']; missing=[f for f in files if not Path(f).exists()]; print('missing=', missing); print('issues=', sum(1 for line in Path('ISSUES.md').read_text().splitlines() if line.startswith('## Issue ')))"
```

## Review Gates

- Do not add source code.
- Do not add package manager manifests.
- Do not add CI YAML during this task; describe CI only in documentation.
- Do not commit changes.
- Do not claim protocol behavior is implemented.
- Do not document Enterprise-only features as required for the open source edition.

## Rollback Points

- If root `AGENTS.md` replacement loses the Trellis block, restore that block immediately.
- If documentation becomes too large for one reviewable change, split by the groups above, starting with product and architecture docs.
- If validation finds fewer than 100 issues, append issue drafts before final reporting.
