# TOML Config Loading Design

## Objective

Extend `sql-lens-config` from a typed model crate into a small configuration loading crate for startup TOML files.

The crate should still not apply configuration to running services. It only reads and parses startup config.

## Crate Boundary

Modify only:

- `crates/sql-lens-config/Cargo.toml`
- `crates/sql-lens-config/src/lib.rs`

Expected lockfile update:

- `Cargo.lock`

Task metadata and planning files live under:

- `.trellis/tasks/07-06-implement-toml-config-loading/`

## Public API

Add inherent methods to `SqlLensConfig`:

```rust
impl SqlLensConfig {
    pub fn from_path(path: impl AsRef<std::path::Path>) -> Result<Self, ConfigLoadError>;
    pub fn from_toml_str(input: &str) -> Result<Self, ConfigLoadError>;
}
```

`from_path`:

1. Reads the file with `std::fs::read_to_string`.
2. Parses content with the same TOML parsing path used by `from_toml_str`.
3. Includes the path in read and parse errors.

`from_toml_str`:

1. Parses TOML from an in-memory string.
2. Returns parse errors without a path.

## Error Contract

Add:

```rust
pub enum ConfigLoadError {
    Read { path: PathBuf, source: std::io::Error },
    Parse { path: Option<PathBuf>, source: toml::Error },
}
```

The exact TOML error type should match the selected `toml` version. With the current `toml` docs, `toml::from_str` returns `toml::Error`.

Implement:

- `Debug`
- `Display`
- `std::error::Error`

Do not derive `Clone`, `PartialEq`, or `Eq` for the error because underlying IO and TOML error types are not stable value contracts.

Tests should assert error variants with `matches!` rather than comparing full error strings.

## Serde Defaults And Strictness

Use serde defaults so partial TOML files can rely on `SqlLensConfig::default()` values:

```rust
#[serde(default)]
```

Apply to config structs so missing nested fields are filled from section defaults.

Use strict unknown-field rejection:

```rust
#[serde(deny_unknown_fields)]
```

Rationale:

- Defaults make local config ergonomic.
- Unknown-field rejection catches typos early.
- Required semantic validation is still out of scope and belongs to Issue 010.

## Dependency Design

Add:

```toml
toml = { version = "0.9", default-features = false, features = ["parse", "serde"] }
```

Do not add `tempfile`. Test file helpers can use `std::env::temp_dir`, `std::fs`, and a unique directory name from process id plus a monotonic counter.

## Data Flow

```text
Path
  -> fs::read_to_string
  -> String
  -> toml::from_str::<SqlLensConfig>
  -> SqlLensConfig
```

Error flow:

```text
read_to_string error -> ConfigLoadError::Read { path, source }
toml parse error     -> ConfigLoadError::Parse { path, source }
string parse error   -> ConfigLoadError::Parse { path: None, source }
```

## Compatibility

- Existing default construction remains unchanged.
- Existing serde traits remain available.
- TOML field names follow the existing config contract, including `storage.type` via `#[serde(rename = "type")]`.
- The first implementation target remains MySQL-compatible defaults while preserving future protocol enum variants.

## Risks

- If `toml = "0.9"` has a different public error type than expected, adjust the error variant source type during implementation and keep the structured error contract.
- `deny_unknown_fields` may be stricter than some config systems, but it is appropriate before SQL Lens has config validation diagnostics.
- Minimal TOML features should be verified with `cargo check`; if feature names changed, use the narrowest working feature set.

## Rollback

Rollback by removing:

- The `toml` dependency.
- Loading methods.
- `ConfigLoadError`.
- TOML-specific tests.
- Any lockfile entries introduced only by `toml`.
