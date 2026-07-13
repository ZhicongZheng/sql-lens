# Integrate Protocol Adapter Registry Into App Runtime

## Goal

Replace the app's hard-coded MySQL adapter construction with the existing object-safe protocol registry so runtime composition has one adapter selection boundary.

## Requirements

- Register the built-in MySQL adapter during app startup.
- Resolve the adapter from the configured protocol/target rather than constructing it in the forwarding loop.
- Return a clear startup/runtime error for unsupported protocols.
- Preserve MySQL behavior and protocol-neutral event contracts.
- Keep `sql-lens-config` independent from protocol crates.

## Acceptance Criteria

- MySQL runtime starts with the registry and captures existing query/prepared events unchanged.
- A configured unsupported protocol fails validation or startup clearly instead of falling through to MySQL.
- Multiple configured targets resolve the adapter by each target's protocol.
- Registry selection and runtime forwarding have focused tests.

## Out Of Scope

- Implementing PostgreSQL, ClickHouse, or SQLite client/server adapters.
- Dynamic plugin-provided protocol adapters.
