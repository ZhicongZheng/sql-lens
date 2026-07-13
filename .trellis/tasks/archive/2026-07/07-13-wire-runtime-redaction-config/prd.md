# Wire Configured Redaction Policy Through Runtime Storage And API

## Goal

Make `SqlLensConfig.redaction` the single runtime redaction policy so configured masks, parameter names, SQL patterns, and enable/disable behavior are honored at every persistence and broadcast boundary.

## Requirements

- Translate `RedactionConfig` into the core `RedactionPolicy` at app startup.
- Apply the policy to Ring Buffer, SQLite persistence, live WebSocket events, and API responses where data crosses a trust boundary.
- Never persist or broadcast unredacted sensitive values when redaction is enabled.
- Preserve disabled-redaction behavior only when explicitly configured.
- Keep redaction idempotent and avoid duplicating policy logic in storage/API layers.

## Acceptance Criteria

- Custom mask and parameter names affect Ring Buffer and SQLite output.
- SQL patterns are applied according to the documented pattern semantics.
- Disabled redaction leaves events unchanged through all configured runtime paths.
- WebSocket and REST detail responses cannot expose values that storage has redacted.
- Tests cover sensitive SQL, parameter values, custom policies, and SQLite persistence.

## Out Of Scope

- Authentication packet persistence or logging.
- A new redaction language or regex engine unless the current contract requires it.
