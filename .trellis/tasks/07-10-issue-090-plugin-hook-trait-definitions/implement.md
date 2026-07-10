# Issue 090 Implementation Plan

1. Read the backend directory, quality, error-handling, and plugin guidance
   before editing Rust code.
2. Add the path dependency from `sql-lens-plugin` to `sql-lens-core`.
3. Implement `PluginError`, `PluginResult`, and the five public object-safe
   hook traits in `crates/sql-lens-plugin/src/lib.rs`.
4. Add focused unit tests for payload construction, hook invocation through
   trait objects, successful callbacks, and isolated callback errors.
5. Update `PLUGIN.md` with the concrete Rust contract and the redaction/error
   boundary rules.
6. Run the narrow validation first:
   - `rtk cargo fmt --check`
   - `rtk cargo test -p sql-lens-plugin`
7. Run the broader backend validation:
   - `rtk cargo test --workspace`
   - `rtk cargo clippy --workspace --all-targets -- -D warnings`
8. Review the diff for accidental changes to the user’s existing frontend or
   local runtime files, then run the Trellis quality check before archiving.

## Risk And Rollback

- The main compatibility risk is selecting hook signatures that future runtime
  dispatch cannot use without cloning or exposing protocol-specific types.
- The implementation must remain confined to `sql-lens-plugin`, its Cargo
  manifest, `PLUGIN.md`, and the task artifacts.
- Rollback is a file-level revert of the new plugin dependency, contract code,
  tests, and documentation; no runtime data or database migration is involved.

## Review Gate

Do not run `task.py start` until the user has reviewed and approved `prd.md`,
`design.md`, and `implement.md`. After activation, implementation happens in
the main session under the inline Trellis workflow.
