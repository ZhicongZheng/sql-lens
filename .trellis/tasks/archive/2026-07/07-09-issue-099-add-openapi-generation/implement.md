# Issue 099 Implementation Plan

1. Read API/backend specs and current REST handler DTOs.
2. Add OpenAPI generation dependencies to `sql-lens-api`.
3. Add `ToSchema` derives to REST request/response DTOs.
4. Add OpenAPI path metadata for current REST endpoints.
5. Add a `sql-lens-api` OpenAPI aggregate and YAML generation function.
6. Add `examples/generate-openapi.rs` to print YAML.
7. Generate `docs/openapi/sql-lens.v1.yaml`.
8. Add a staleness test comparing generated YAML to the committed file.
9. Update docs/spec if the OpenAPI contract introduces new conventions.
10. Validate:
   - `rtk cargo fmt --check`
   - `rtk cargo test -p sql-lens-api`
   - `rtk cargo test --workspace`
   - `rtk cargo clippy --workspace --all-targets -- -D warnings`

## Risk Notes

- DTOs with nested metadata or enum-like string fields may need explicit schema aliases.
- Handler annotations should avoid changing route behavior.
- Generated YAML ordering must be deterministic enough for a direct file comparison.

## Rollback Points

- If full code-first generation becomes too broad, keep the generator module and limit the first pass to DTO schemas plus documented REST paths.
- If YAML formatting is unstable, compare parsed OpenAPI JSON values in the staleness test and still commit the generated YAML.
