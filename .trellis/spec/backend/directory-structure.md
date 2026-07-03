# Backend Directory Structure

> Backend code organization for SQL Lens.

## Overview

SQL Lens backend is Rust-first and organized as a Cargo workspace. Each crate owns one clear runtime or domain boundary. Protocol-specific crates depend on shared capture models, never the reverse.

## Directory Layout

```text
crates/
├── sql-lens-core/
├── sql-lens-proxy/
├── sql-lens-protocol/
├── sql-lens-protocol-mysql/
├── sql-lens-storage/
├── sql-lens-api/
├── sql-lens-plugin/
└── sql-lens-app/
```

## Workspace Manifest Contract

Root `Cargo.toml` is a virtual workspace manifest.

Required workspace settings:

- `resolver = "3"`.
- `edition = "2024"` through `[workspace.package]`.
- `rust-version = "1.85"` through `[workspace.package]`.
- `license = "Apache-2.0"` through `[workspace.package]`.

Member packages inherit shared package metadata instead of repeating it.

`sql-lens-app` is the application package and exposes the user-facing binary named `sql-lens`.

## Module Ownership

- `sql-lens-core`: protocol-neutral domain models such as SQL events, connections, parameters, timings, result summaries, error summaries, and protocol metadata containers.
- `sql-lens-proxy`: TCP listener, backend dialing, session lifecycle, bidirectional forwarding, shutdown, and backpressure coordination.
- `sql-lens-protocol`: protocol adapter traits, adapter registry, and shared adapter contracts.
- `sql-lens-protocol-mysql`: MySQL-compatible packet framing, handshake observation, command parsing, prepared statement lifecycle, parameter decoding, and error packet mapping.
- `sql-lens-storage`: ring buffer, SQLite, future DuckDB, retention, query filters, and statistics helpers.
- `sql-lens-api`: REST handlers, WebSocket handlers, API error mapping, and OpenAPI schema generation.
- `sql-lens-plugin`: hook traits, exporter traits, plugin lifecycle, and plugin safety boundaries.
- `sql-lens-app`: CLI, config loading, logging setup, runtime startup, and graceful shutdown.

## Dependency Rules

- Core must not depend on protocol-specific crates.
- Proxy must not contain SQL rendering logic.
- API must not parse protocol packets.
- Storage must not depend on UI or API handlers.
- Plugin payloads must use stable, protocol-neutral models plus optional metadata.
- MySQL-only details must live under protocol metadata or the MySQL adapter crate.

## Naming Conventions

- Crates use `sql-lens-*`.
- Rust modules use `snake_case`.
- Rust types use `PascalCase`.
- JSON fields use `snake_case`.
- Protocol adapter names use lowercase identifiers such as `mysql`, `postgresql`, `clickhouse`, and `sqlite`.

## Common Mistakes

- Do not add MySQL-specific fields directly to shared event structs.
- Do not block TCP forwarding on storage, UI, exporters, or plugin hooks.
- Do not introduce generic multi-protocol abstractions before a second real adapter needs them.
- Do not treat SQLite as a TCP proxy target; it requires a separate tracing or driver integration design.
