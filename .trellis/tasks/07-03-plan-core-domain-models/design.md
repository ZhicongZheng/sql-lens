# Core Domain Models Design

## Objective

Define the initial protocol-neutral domain contract in `sql-lens-core` without pulling in runtime, protocol parser, storage, or API handler behavior.

## Design Principles

- Keep shared types protocol-neutral.
- Prefer small explicit newtypes over raw strings and numbers on public structs.
- Avoid dependencies until a specific contract needs them.
- Use typed metadata instead of arbitrary JSON for the first slice.
- Keep source modules aligned with domain concepts.

## Dependency Policy

`sql-lens-core` adds:

```toml
serde = { version = "1.0", features = ["derive"] }
```

Do not add:

- `serde_json`
- `time`
- `uuid`
- HTTP framework crates
- storage crates

Rationale:

- `serde` is required because these models are future API, storage, WebSocket, and plugin payloads.
- `time` and `uuid` can be introduced later when runtime generation and storage formats are settled.
- `serde_json::Value` would make metadata too loose for the first contract.

## Module Design

### `ids.rs`

Core-owned ID newtypes:

- `SqlEventId`
- `ConnectionId`
- `StatementId`

Each wraps `String`.

### `time.rs`

Core-owned time newtypes:

- `Timestamp(String)` for RFC3339-like timestamps.
- `DurationMillis(u64)` for elapsed durations.

These are deliberately lightweight until runtime and storage code establish stricter requirements.

### `metadata.rs`

Protocol-neutral metadata types:

- `ProtocolName(String)`
- `DatabaseType(String)`
- `ProtocolMetadata`
- `MetadataField`
- `MetadataValue`

`MetadataValue` supports:

- `String`
- `Integer`
- `Unsigned`
- `Float`
- `Boolean`

No nested objects in the first slice.

### `event.rs`

Capture model types:

- `SqlEvent`
- `SqlEventKind`
- `CaptureStatus`
- `ConnectionInfo`
- `ConnectionState`
- `PreparedStatementInfo`
- `SqlParameter`
- `SqlParameterValue`
- `QueryTiming`
- `ResultSummary`

`SqlEvent` owns the common capture event fields used by storage/API/UI. Optional fields should represent values that are not always known, such as expanded SQL, user, database, row counts, and error details.

### `error.rs`

Error contract types:

- `ApiError`
- `ApiErrorCode`
- `ErrorSummary`

`ApiErrorCode` should include documented API error codes:

- `BadRequest`
- `Unauthorized`
- `Forbidden`
- `NotFound`
- `Conflict`
- `RateLimited`
- `Internal`
- `StorageUnavailable`
- `ProxyNotReady`

`ErrorSummary` captures database/protocol error details when available without forcing HTTP concepts into SQL capture events.

## Re-exports

`lib.rs` should declare modules and re-export public model types so downstream crates can import from `sql_lens_core`.

## Testing Design

Unit tests should stay lightweight:

- Construct representative models.
- Assert a few key fields.
- Use generic helper functions to prove `Serialize` / `Deserialize` trait availability.
- Do not serialize to JSON strings.

## Compatibility

This design supports MySQL-compatible protocol first while preserving future PostgreSQL, SQLite integration, and ClickHouse extension points through protocol-neutral fields and typed metadata.

## Risks

- Too many optional fields can weaken contract clarity.
- Too many strict fields can block partially observed protocol events.
- Metadata can become a dumping ground if future tasks do not document protocol-specific subfields.

## Rollback

Rollback is limited to reverting changes in `crates/sql-lens-core` and removing the `serde` dependency from that crate.

