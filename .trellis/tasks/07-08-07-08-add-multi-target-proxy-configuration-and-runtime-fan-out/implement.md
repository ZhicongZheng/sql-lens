# Implement — Multi-Target Proxy Configuration and Runtime Fan-Out

## Steps

1. Read backend specs, frontend architecture spec, and current config/runtime
   code.
2. Add config target model:
   - `ProxyTargetConfig`
   - effective target conversion preserving old `[proxy]` + `[backend]`
3. Add validation:
   - missing target name/listen/backend address
   - unsupported protocol
   - duplicate target names
   - duplicate listen addresses
4. Add tests for config defaults, TOML parsing, validation errors, and backward
   compatibility.
5. Refactor app runtime helpers:
   - target runtime config struct
   - one listener task per target
   - shared `ApiState`
   - graceful shutdown of all target tasks
6. Ensure connection/event metadata contains correct `database_type` and target
   identity.
7. Add narrow runtime tests with two local targets.
8. Update backend/frontend architecture specs if implementation details refine
   the planning contract.
9. Validate:
   - `rtk cargo fmt --check`
   - `rtk cargo test -p sql-lens-config`
   - `rtk cargo test -p sql-lens-app`
   - `rtk cargo test --workspace`
   - `rtk cargo clippy --workspace --all-targets -- -D warnings`

## Candidate Files

- `crates/sql-lens-config/src/model.rs`
- `crates/sql-lens-config/src/validation.rs`
- `crates/sql-lens-config/src/tests.rs`
- `crates/sql-lens-app/src/lib.rs`
- `CONFIG.md`
- `ARCHITECTURE.md`
- `ISSUES.md`
- `.trellis/spec/backend/directory-structure.md`
- `.trellis/spec/frontend/directory-structure.md`
- `.trellis/spec/backend/quality-guidelines.md`

## Rollback

Keep the old single-target config path until multi-target behavior is fully
verified. If runtime fan-out proves too broad, land config/effective-target
model first and split listener fan-out into a follow-up.
