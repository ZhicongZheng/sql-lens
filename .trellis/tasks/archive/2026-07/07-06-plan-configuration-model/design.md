# Configuration Model Design

## Objective

Create a standalone `sql-lens-config` crate that owns the typed configuration contract for SQL Lens startup configuration.

The crate should define structs, enums, defaults, and lightweight tests only.

## Crate Boundary

Add a workspace member:

```text
crates/sql-lens-config/
```

Responsibility:

- Own runtime configuration model types.
- Own defaults for startup configuration.
- Prepare for future TOML deserialization through serde derives.

Non-responsibility:

- Reading files.
- Reading environment variables.
- Validating runtime constraints.
- Applying hot reload.
- Starting services.

## Dependency Policy

Allowed:

```toml
serde = { version = "1.0", features = ["derive"] }
```

Not allowed in this task:

- `toml`
- validation crates
- `sql-lens-core`
- runtime crates
- database crates

## Source Layout

Recommended layout:

```text
crates/sql-lens-config/src/
  lib.rs
```

Keep this first slice in one file unless it becomes unwieldy. The crate is a single coherent model surface. Splitting by section can happen later when loader/validation code arrives.

## Top-Level Model

Use a top-level config struct:

```rust
pub struct SqlLensConfig {
    pub proxy: ProxyConfig,
    pub backend: BackendConfig,
    pub tls: TlsConfig,
    pub web: WebConfig,
    pub storage: StorageConfig,
    pub retention: RetentionConfig,
    pub logging: LoggingConfig,
    pub redaction: RedactionConfig,
    pub auth: AuthConfig,
    pub replay: ReplayConfig,
    pub plugins: PluginsConfig,
}
```

Each section implements `Default`.

`SqlLensConfig::default()` composes all section defaults.

## Enum Strategy

Use config-owned enums instead of core domain model types:

- `Protocol`
- `DatabaseType`
- `TlsMode`
- `StorageType`
- `LoggingLevel`
- `LoggingFormat`
- `AuthMode`
- `RetentionDropPolicy`
- `CaptureMode`

Rationale:

- Config is a runtime contract, not a capture event contract.
- Avoid premature dependency from config to core.
- Keep config option sets explicit and easy to validate later.

## Hybrid Configuration Future

Long-term source strategy:

- TOML: startup and foundational system configuration.
- SQLite: future runtime settings mutated through UI/API.
- Hot reload: separate watcher/diff/apply layer.

This task should not implement hot reload or mark field-level reloadability. It should keep models simple enough for future reload policies to inspect.

## Tests

Lightweight tests:

- Construct `SqlLensConfig::default()`.
- Assert key defaults from `CONFIG.md`.
- Assert serde traits are available for top-level and representative section types.

Do not use `serde_json` or TOML roundtrips in this task.

## Risks

- Adding TOML loading now would blur Issue 007 and Issue 008.
- Depending on `sql-lens-core` would couple runtime config to capture event model too early.
- Field-level reloadability metadata now would likely be speculative.

## Rollback

Rollback by removing:

- Workspace member entry.
- `crates/sql-lens-config`.
- Any Cargo lock changes for serde if no longer needed elsewhere.

