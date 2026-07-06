# Backend Quality Guidelines

> Code quality standards for backend development.

## Scenario: Core Domain Model Contracts

### 1. Scope / Trigger

- Trigger: `sql-lens-core` defines cross-layer public contract types used by proxy, protocol adapters, storage, API, WebSocket, plugins, and UI-facing schemas.
- These types are not implementation details. Changes to them can ripple across multiple layers.
- Core models must stay protocol-neutral. Protocol-specific fields belong in typed metadata.

### 2. Signatures

Core model modules live under:

```text
crates/sql-lens-core/src/
  ids.rs
  time.rs
  metadata.rs
  event.rs
  error.rs
```

Public types are re-exported from `lib.rs`.

Required dependency policy for the first core model layer:

```toml
serde = { version = "1.0", features = ["derive"] }
```

Do not add these dependencies to `sql-lens-core` without a new design decision:

- `serde_json`
- `time`
- `uuid`
- async runtime crates
- HTTP framework crates
- database or storage crates

### 3. Contracts

Public model types should derive:

- `Debug`
- `Clone`
- `PartialEq`
- `serde::Serialize`
- `serde::Deserialize`
- `Eq` only where practical

Do not force `Eq` onto types containing floating-point values or metadata that can contain floating-point values.

ID and time values must use core-owned newtypes:

- `SqlEventId`
- `ConnectionId`
- `StatementId`
- `RequestId`
- `Timestamp`
- `DurationMillis`

Protocol metadata must use typed fields:

```rust
pub struct ProtocolMetadata {
    pub protocol: ProtocolName,
    pub fields: Vec<MetadataField>,
}
```

Do not use arbitrary JSON for protocol metadata in the first core contract.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| A shared model needs protocol-specific data | Put it under `ProtocolMetadata`, not as a top-level field |
| A model contains `f64` directly or indirectly | Do not derive `Eq` |
| A new public model is added | Re-export it from `lib.rs` and add a construction or trait test |
| A new dependency is proposed for `sql-lens-core` | Justify it in the task design before implementation |
| API-style errors are needed | Use `ApiError` and `ApiErrorCode`; do not invent per-layer response shapes |

### 5. Good/Base/Bad Cases

Good:

- `SqlEvent` uses `ProtocolName`, `DatabaseType`, `ConnectionId`, `Timestamp`, and `ProtocolMetadata`.
- MySQL statement IDs are represented inside metadata or a protocol-neutral statement ID wrapper.
- `ApiErrorCode` is shared and stable.

Base:

- A new optional field is added to `SqlEvent` only after checking `PRD.md`, `STORAGE.md`, and `API.md`.
- A new enum variant is added with tests and downstream impact noted.

Bad:

- Adding `mysql_statement_id: u32` as a top-level `SqlEvent` field.
- Adding `serde_json::Value` to `ProtocolMetadata` without a design update.
- Deriving `Eq` for a type that contains `f64`.

### 6. Tests Required

For public core model changes:

- Construct representative instances in unit tests.
- Assert important fields.
- Add compile-time trait checks for `Serialize` and `Deserialize`.
- Run `cargo fmt --check`.
- Run `cargo check --workspace`.
- Run `cargo test --workspace`.
- Run `cargo clippy --workspace --all-targets -- -D warnings` before committing backend model changes.

### 7. Wrong vs Correct

#### Wrong

```rust
pub struct SqlEvent {
    pub mysql_statement_id: Option<u32>,
    pub metadata: serde_json::Value,
}
```

#### Correct

```rust
pub struct SqlEvent {
    pub metadata: ProtocolMetadata,
}

pub struct ProtocolMetadata {
    pub protocol: ProtocolName,
    pub fields: Vec<MetadataField>,
}
```

## Scenario: Config Loading Contracts

### 1. Scope / Trigger

- Trigger: `sql-lens-config` owns startup configuration structs, serde-compatible config shape, default values, and startup TOML parsing.
- Config loading is a boundary contract for CLI startup, validation, logging setup, runtime startup, and future hot reload.
- Config parsing must stay separate from semantic validation and runtime apply logic.

### 2. Signatures

Public TOML loading APIs live on `SqlLensConfig`:

```rust
impl SqlLensConfig {
    pub fn from_path(path: impl AsRef<std::path::Path>) -> Result<Self, ConfigLoadError>;
    pub fn from_toml_str(input: &str) -> Result<Self, ConfigLoadError>;
}
```

Structured load errors use:

```rust
pub enum ConfigLoadError {
    Read {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    Parse {
        path: Option<std::path::PathBuf>,
        source: toml::de::Error,
    },
}
```

### 3. Contracts

- `from_path` reads a file and parses it as TOML.
- `from_toml_str` parses already-loaded TOML content.
- Missing sections and fields use the existing `Default` implementations.
- Unknown sections and fields are rejected with `#[serde(deny_unknown_fields)]`.
- The config crate may depend on `serde` and `toml` for this layer.
- The config crate must not depend on CLI, async runtime, HTTP, database, watcher, or proxy crates for loading.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| Config file cannot be read | Return `ConfigLoadError::Read` with the path and IO source |
| TOML from a path cannot be parsed | Return `ConfigLoadError::Parse` with `Some(path)` |
| TOML from a string cannot be parsed | Return `ConfigLoadError::Parse` with `None` |
| Section or field is missing | Use the section or field default |
| Section or field is unknown | Return a parse/deserialization error |
| Required semantic value is empty or unsupported at runtime | Leave to the later config validation layer |

### 5. Good/Base/Bad Cases

Good:

- A local config file contains only `[proxy] listen = "127.0.0.1:3308"` and the rest comes from defaults.
- A misspelled field like `lissten` fails during TOML deserialization.
- A missing file reports `ConfigLoadError::Read`.

Base:

- A caller uses `SqlLensConfig::from_toml_str` in tests and `SqlLensConfig::from_path` in CLI code.
- Validation later rejects semantically invalid values after TOML parsing succeeds.

Bad:

- Silently ignoring unknown config fields.
- Starting services from `sql-lens-config`.
- Adding environment overrides, hot reload, or CLI argument parsing inside `sql-lens-config`.

### 6. Tests Required

For config loading changes:

- Valid TOML file loads from a path.
- TOML string parsing works.
- Partial TOML falls back to defaults.
- Unknown top-level sections and nested fields fail.
- Invalid TOML returns `ConfigLoadError::Parse`.
- Missing files return `ConfigLoadError::Read`.
- `ConfigLoadError` implements `Display` and `std::error::Error`.

### 7. Wrong vs Correct

#### Wrong

```rust
pub fn load_config(path: &str) -> SqlLensConfig {
    toml::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}
```

#### Correct

```rust
pub fn load_config(path: impl AsRef<std::path::Path>) -> Result<SqlLensConfig, ConfigLoadError> {
    SqlLensConfig::from_path(path)
}
```

## Forbidden Patterns

- Do not put MySQL-only fields directly on shared core models.
- Do not add arbitrary JSON metadata without a task-level design update.
- Do not add runtime, HTTP, database, or storage dependencies to `sql-lens-core` for model-only work.
- Do not leave new public models without construction tests.

## Code Review Checklist

- Does the change preserve protocol neutrality?
- Are new public types re-exported from `lib.rs`?
- Are dependencies still minimal?
- Are serde traits available where required?
- Is `Eq` only derived where all fields can support it?
- Do tests cover representative construction and trait availability?
