# Config Validation Design

## Objective

Add a small semantic validation layer to `sql-lens-config`.

This layer checks whether a parsed or manually constructed `SqlLensConfig` is acceptable for the current SQL Lens startup phase. It does not start services or validate every documented future rule.

## Crate Boundary

Modify:

- `crates/sql-lens-config/src/lib.rs`

No new dependencies are expected.

Task metadata and planning files live under:

- `.trellis/tasks/07-06-add-config-validation/`

## Public API

Add:

```rust
impl SqlLensConfig {
    pub fn validate(&self) -> Result<(), ConfigValidationError>;
}
```

Add structured validation types:

```rust
pub struct ConfigValidationError {
    pub violations: Vec<ConfigValidationViolation>,
}

pub enum ConfigValidationViolation {
    MissingProxyListen,
    MissingBackendAddress,
    UnsupportedProtocol { protocol: Protocol },
}
```

Derive `Debug`, `Clone`, `PartialEq`, and `Eq` for validation types where practical. Implement `Display` and `std::error::Error` for `ConfigValidationError`.

## Validation Rules

Initial rules:

- `proxy.listen.trim().is_empty()` -> `MissingProxyListen`.
- `backend.address.trim().is_empty()` -> `MissingBackendAddress`.
- `proxy.protocol != Protocol::MySql` -> `UnsupportedProtocol`.

Why `Protocol::MySql` only:

- SQL Lens is multi-protocol by architecture.
- The first implementation target is the MySQL-compatible protocol family.
- Future protocol enum variants are allowed to keep config shape extensible, but should not pass runtime validation until their adapters exist.

## Error Shape

Validation should collect all currently detectable violations:

```text
SqlLensConfig
  -> validate()
  -> Vec<ConfigValidationViolation>
  -> Ok(()) if empty
  -> Err(ConfigValidationError { violations }) if not empty
```

Collecting all violations improves developer feedback without introducing much complexity.

## Data Flow

```text
TOML file/string
  -> SqlLensConfig::from_path / from_toml_str
  -> SqlLensConfig
  -> SqlLensConfig::validate
  -> CLI/runtime startup later
```

TOML parsing remains responsible for syntax, enum value deserialization, defaults, and unknown field rejection.

Validation is responsible for semantic startup readiness.

## Compatibility

- Existing config loading and default behavior remain unchanged.
- `SqlLensConfig::default()` should validate successfully.
- Manually constructed configs can be validated without reading TOML.
- Future tasks can extend `ConfigValidationViolation` with storage, TLS, auth, and protocol-registry rules.

## Risks

- Rejecting future protocol enum variants may feel strict, but it prevents users from starting SQL Lens with adapters that are not implemented yet.
- Address syntax validation is intentionally deferred; otherwise this task grows into network configuration parsing.
- Protocol registry integration is deferred because the registry crate does not yet own runtime adapter availability.

## Rollback

Rollback by removing:

- `SqlLensConfig::validate`.
- `ConfigValidationError`.
- `ConfigValidationViolation`.
- Validation tests.
