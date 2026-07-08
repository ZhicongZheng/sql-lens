# Backend Directory Structure

> Backend code organization for SQL Lens.

## Overview

SQL Lens backend is Rust-first and organized as a Cargo workspace. Each crate owns one clear runtime or domain boundary. Protocol-specific crates depend on shared capture models, never the reverse.

## Directory Layout

```text
crates/
├── sql-lens-core/
├── sql-lens-capture/
├── sql-lens-config/
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
- `sql-lens-capture`: bounded capture event channel, non-blocking event publisher, capture receiver for future storage/broadcast fan-out, overload policy, and dropped-event counters.
- `sql-lens-config`: startup configuration structs, configuration enums, defaults, and serde-compatible configuration shape.
- `sql-lens-proxy`: TCP listener, backend dialing, session lifecycle, bidirectional forwarding, shutdown, and backpressure coordination.
- `sql-lens-protocol`: protocol adapter traits, adapter registry, and shared adapter contracts.
- `sql-lens-protocol-mysql`: MySQL-compatible packet framing, handshake observation, command parsing, prepared statement lifecycle, parameter decoding, and error packet mapping.
- `sql-lens-storage`: ring buffer, SQLite, future DuckDB, retention, query filters, and statistics helpers.
- `sql-lens-api`: REST handlers, WebSocket handlers, API error mapping, and OpenAPI schema generation.
- `sql-lens-plugin`: hook traits, exporter traits, plugin lifecycle, and plugin safety boundaries.
- `sql-lens-app`: CLI, config loading, logging setup, runtime startup, and graceful shutdown.

## Multi-Target Proxy Architecture

SQL Lens may run multiple explicitly configured proxy targets in one process.
Each target owns exactly one listener and one backend address. This supports
debugging applications that talk to several MySQL-compatible surfaces, such as
MySQL and StarRocks, without making SQL Lens a database middleware.

Backend ownership rules:

- `sql-lens-config` owns the target configuration shape and validation.
- `sql-lens-app` owns expanding effective targets and starting one listener task
  per target.
- `sql-lens-proxy` remains a TCP listener/dialer/forwarding crate; it must not
  choose a backend dynamically from SQL text, user, database, SNI, or packet
  contents.
- Protocol adapters remain per-connection observers and must not own target
  routing policy.
- Storage/API consume already-classified target identity from shared event or
  metadata contracts; they must not infer target identity from port strings.

Forbidden for multi-target support:

- SQL rewrite, sharding, read/write splitting, failover, load balancing, or
  backend policy enforcement.
- One listener multiplexing arbitrary client traffic to multiple backends.
- MySQL-specific target identity fields in protocol-neutral core models.

## Crate Root Convention

`src/lib.rs` should stay thin once a crate has more than one real responsibility. Prefer:

```rust
mod domain_module;
mod second_domain_module;

pub use domain_module::{PublicType, PublicTrait};
pub use second_domain_module::OtherPublicType;
```

Rules:

- Keep protocol-neutral public contracts re-exported from `lib.rs`.
- Move implementation into domain-named modules once the crate has clear subdomains.
- Keep tests next to the module they exercise or in `src/tests.rs` when they span multiple crate modules.
- Do not split placeholder crates that only contain crate-level documentation.
- Do not create empty module files just to mirror future architecture.

Current module boundaries:

- `sql-lens-config`: `model`, `loading`, `validation`, `error`, and tests.
- `sql-lens-capture`: `pipeline`.
- `sql-lens-protocol`: `adapter`, `registry`, and tests.
- `sql-lens-proxy`: `listener`, `dialer`, `forwarding`, `lifecycle`, `shutdown`, and cross-module tests.
- `sql-lens-storage`: `ring_buffer`, `live_statistics`, and crate-level re-exports.

## Dependency Rules

- Core must not depend on protocol-specific crates.
- Capture must not depend on proxy, protocol, storage, API, plugin, app, database, HTTP, or exporter crates.
- Protocol contracts should depend on core only until a task explicitly wires registry or runtime composition.
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
- Do not put capture channel primitives inside proxy, protocol, storage, or API crates.
- Do not introduce generic multi-protocol abstractions before a second real adapter needs them.
- Do not treat SQLite as a TCP proxy target; it requires a separate tracing or driver integration design.
