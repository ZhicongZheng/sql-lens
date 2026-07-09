# Issue 099 Design: Code-First OpenAPI Generation

## Scope

Add OpenAPI generation to `sql-lens-api` and commit the generated v1 YAML at `docs/openapi/sql-lens.v1.yaml`.

## Approach

Use `utoipa` as the Rust code-first OpenAPI generator:

- Derive `ToSchema` for API DTOs that already define JSON response/request contracts.
- Add path metadata for existing REST handlers or lightweight OpenAPI-only path functions.
- Add an `OpenApi` aggregate type in `sql-lens-api`.
- Enable YAML serialization through the `utoipa` YAML feature.
- Add an example or small binary command that prints the generated YAML.

This keeps the source of truth near backend API types and avoids a hand-maintained YAML file drifting from Rust DTOs.

## Data Flow

```text
REST DTO structs + path annotations
  -> sql-lens-api OpenApi aggregate
  -> deterministic YAML generator command
  -> docs/openapi/sql-lens.v1.yaml
  -> staleness test compares generated YAML with committed YAML
```

## Boundaries

- `sql-lens-api` owns OpenAPI schema generation because it owns REST handlers and API DTOs.
- `sql-lens-app` should not generate OpenAPI; it only composes runtime services.
- `sql-lens-config`, `sql-lens-core`, and `sql-lens-storage` should not depend on OpenAPI libraries for this task.
- Frontend code should consume the generated artifact later; it is not modified here.

## Contracts

- Generated YAML path: `docs/openapi/sql-lens.v1.yaml`.
- Suggested refresh command:

```bash
rtk cargo run -p sql-lens-api --example generate-openapi > docs/openapi/sql-lens.v1.yaml
```

- Staleness check lives in `sql-lens-api` tests and compares generated YAML text to the committed file.
- The OpenAPI version is v1 and should align with the existing `/api/v1` route prefix.

## Trade-Offs

- `utoipa` adds proc-macro/schema dependencies to `sql-lens-api`, but avoids broad custom YAML construction.
- Annotating DTOs creates some mechanical churn, but makes schema drift visible in Rust review.
- A repo-local staleness test satisfies the current issue without introducing GitHub Actions before Issue 094 is implemented.

## Compatibility

- No runtime API behavior changes.
- No frontend behavior changes.
- The generated file is a public contract; future endpoint changes should update it in the same commit.
