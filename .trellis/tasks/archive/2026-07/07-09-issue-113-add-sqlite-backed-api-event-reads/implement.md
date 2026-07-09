# Issue 113 Implementation Plan

1. Read relevant backend storage/API/app specs and existing SQLite query code.
2. Inspect current `ApiState`, SQL event handlers, export handler, replay
   preview handler, and app runtime storage selection.
3. Add a narrow API event read-source type with ring-buffer and SQLite variants.
4. Move list/detail/export/replay event lookup to the read-source boundary.
5. Add SQLite row-to-response mapping, including parameter value decoding and
   metadata JSON handling.
6. Add SQLite cursor encode/decode support with a distinct prefix.
7. Wire `sql-lens-app` runtime so SQLite storage config installs the SQLite
   read source in `ApiState`.
8. Add focused API tests for SQLite-backed list/detail/export/replay preview.
9. Add app runtime tests proving SQLite config installs the read source.
10. Update docs/spec if the API storage read-source contract needs to be
    preserved for future agents.
11. Validate:
    - `rtk cargo fmt --check`
    - `rtk cargo test -p sql-lens-storage` if storage code changes
    - `rtk cargo test -p sql-lens-api`
    - `rtk cargo test -p sql-lens-app`
    - `rtk cargo test --workspace`
    - `rtk cargo clippy --workspace --all-targets -- -D warnings`

## Rollback Points

- If SQLite-to-core reconstruction becomes too broad, map rows directly to API
  response DTOs inside `sql-lens-api`.
- If a shared read-source trait creates unnecessary complexity, use a compact
  enum on `ApiState` with explicit match branches.
