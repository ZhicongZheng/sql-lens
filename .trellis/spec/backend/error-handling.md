# Error Handling

> Error handling conventions for SQL Lens backend code.

## Overview

SQL Lens uses explicit Rust error types at crate boundaries and keeps API error
responses protocol-neutral. Errors should preserve useful source information
without logging or exposing secrets, authentication payloads, raw SQL parameters,
or unredacted database error text.

Current examples:

- `crates/sql-lens-config/src/error.rs` defines config load and validation
  errors.
- `crates/sql-lens-api/src/api_error.rs` maps API endpoint errors into JSON
  response envelopes.
- `crates/sql-lens-storage/src/ring_buffer.rs` defines storage filter errors.
- Protocol parsers define local parse error enums in their owning modules.

## Error Types

- Use enums for domain errors when callers need to match specific conditions.
- Derive `Debug`; derive `Clone`, `PartialEq`, and `Eq` when tests or public
  contracts benefit and the fields support them.
- Implement `Display` for human-readable CLI, test, or API messages.
- Implement `std::error::Error` for errors that cross crate boundaries.
- Preserve `source` for wrapped IO, parse, or framework errors.

Example pattern from config loading:

```rust
pub enum ConfigLoadError {
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    Parse {
        path: Option<PathBuf>,
        source: toml::de::Error,
    },
}
```

## Error Handling Patterns

- Return `Result<T, DomainError>` at crate boundaries instead of panicking.
- Keep parser failures non-fatal in observation hot paths unless the adapter
  contract says the caller must stop.
- Use `ok_or_else` to map missing resources to typed not-found errors.
- Keep validation errors structured so tests can assert exact failed fields.
- Avoid catch-all string errors for public contracts.

## API Error Responses

REST handlers return `Result<Json<T>, ApiEndpointError>` when they can fail.
`ApiEndpointError` owns the HTTP status, shared `ApiErrorCode`, message, and
details map.

API errors serialize as:

```json
{
  "error": {
    "code": "BAD_REQUEST",
    "message": "limit must be greater than zero",
    "request_id": "req_...",
    "details": {
      "field": "limit"
    }
  }
}
```

Request IDs are attached by API response middleware. Endpoint code should create
the error with domain details and let `with_request_id` add the final
`request_id` field.

## Validation And Mapping

| Condition | Required behavior |
| --- | --- |
| Config file cannot be read | Return `ConfigLoadError::Read` with path and IO source |
| Config cannot be parsed | Return `ConfigLoadError::Parse` with optional path and TOML source |
| Config semantic validation fails | Return `ConfigValidationError` with all violations found |
| API query parameter is invalid | Return `ApiEndpointError::bad_request` with a field detail |
| API resource is missing | Return `ApiEndpointError::not_found` with identifying details |
| Storage filter range is invalid | Return a storage filter error and map it at the API boundary |

## Tests Required

For error handling changes:

- Exact enum variant tests for validation and parse failures.
- `Display` message tests for public errors.
- `source()` tests when wrapping lower-level errors.
- API response tests for status, code, message, details, and request ID.
- Existing success-path tests remain green.

## Common Mistakes

- Do not log passwords, authentication packet payloads, raw SQL parameters, or
  unredacted database error text.
- Do not add MySQL-specific error fields directly to protocol-neutral API
  responses; put protocol details in metadata.
- Do not convert structured errors into plain strings before crossing an API or
  crate boundary.
- Do not panic for malformed network input; malformed protocol bytes should stay
  non-fatal unless a task explicitly changes that contract.
