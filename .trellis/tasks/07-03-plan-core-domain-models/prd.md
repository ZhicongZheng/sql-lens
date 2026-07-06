# Core Domain Models

## Goal

Implement the first protocol-neutral core domain models in `sql-lens-core`.

This task defines shared capture, connection, prepared statement, metadata, timing, and error contract types that future proxy, protocol, storage, API, WebSocket, plugin, and UI work can depend on.

## User Value

SQL Lens needs one stable language for SQL capture events before any protocol adapter, storage backend, or API can be implemented cleanly. These models prevent MySQL-specific assumptions from leaking into shared layers and give contributors a typed foundation for the next milestones.

## Background

The Rust workspace skeleton is complete.

Repository evidence establishes this as the next foundation layer:

- `ARCHITECTURE.md` assigns shared domain models to `sql-lens-core`: `SqlEvent`, `ConnectionInfo`, `PreparedStatementInfo`, `SqlParameter`, `QueryTiming`, `CaptureStatus`, `ProtocolMetadata`, plus error and result summary types.
- `STORAGE.md` lists SQL event, connection, and prepared statement fields.
- `PRD.md` defines SQL capture required fields and prepared statement expansion requirements.
- `API.md` names public schemas including `SqlEvent`, `SqlEventSummary`, `SqlParameter`, `Connection`, and `ApiError`.
- `AGENTS.md` marks `SqlEvent`, `SqlParameter`, and `ConnectionInfo` as public contracts requiring care.
- The backend spec requires protocol-specific details to stay in metadata and prohibits MySQL-only fields in shared structs.

## Scope

Implement initial `sql-lens-core` model types:

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
- `ErrorSummary`
- `ProtocolName`
- `DatabaseType`
- `ProtocolMetadata`
- `MetadataField`
- `MetadataValue`
- `ApiError`
- `ApiErrorCode`

Add lightweight unit tests for:

- Representative `SqlEvent` construction.
- Representative `ConnectionInfo` construction.
- Prepared statement and parameter construction.
- `ApiError` construction.
- Compile-time `serde::Serialize` / `serde::Deserialize` trait availability.

## Technical Decisions

- Models must be protocol-neutral.
- MySQL-specific fields must not appear directly on shared structs.
- Protocol-specific information belongs under `ProtocolMetadata`.
- `ProtocolMetadata` uses `Vec<MetadataField>` plus typed `MetadataValue`.
- `ProtocolMetadata` does not use arbitrary JSON or nested metadata in this slice.
- Introduce `serde` only:
  - `serde = { version = "1.0", features = ["derive"] }`
- Do not introduce `time`, `uuid`, or `serde_json`.
- IDs, timestamps, and durations use lightweight core-owned newtypes for now.
- `ApiError` and `ApiErrorCode` are included as shared contract types.
- No HTTP handlers, OpenAPI generation, or API routing are included.
- Source is organized by domain modules and public types are re-exported from `lib.rs`.

## Source Layout

```text
crates/sql-lens-core/src/
  lib.rs
  ids.rs
  time.rs
  metadata.rs
  event.rs
  error.rs
```

Responsibilities:

- `ids.rs`: `SqlEventId`, `ConnectionId`, `StatementId`.
- `time.rs`: `Timestamp`, `DurationMillis`.
- `metadata.rs`: `ProtocolName`, `DatabaseType`, `ProtocolMetadata`, `MetadataField`, `MetadataValue`.
- `event.rs`: capture, connection, prepared statement, parameter, timing, result, status, and kind models.
- `error.rs`: API and SQL error summary models.

## Out Of Scope

- Protocol adapter traits.
- MySQL parser details.
- Storage query filters.
- Statistics aggregation types.
- Replay request/response types.
- OpenAPI generation.
- REST or WebSocket handlers.
- Redaction rule engine.
- Serialization roundtrip tests using `serde_json`.
- Introducing `time` or `uuid`.

## Acceptance Criteria

- [ ] `sql-lens-core` depends on `serde` with derive support.
- [ ] Public model types derive `Debug`, `Clone`, `PartialEq`, `Eq` where practical, `Serialize`, and `Deserialize`.
- [ ] ID/time/duration values use core-owned newtypes, not raw values on public structs.
- [ ] `SqlEvent` includes protocol-neutral fields from `PRD.md` and `STORAGE.md`.
- [ ] `ConnectionInfo` includes documented connection fields.
- [ ] `PreparedStatementInfo` includes documented prepared statement fields.
- [ ] `SqlParameter` and `SqlParameterValue` cover common parameter classes without protocol-specific encoding details.
- [ ] `ProtocolMetadata` uses typed metadata fields and does not depend on `serde_json`.
- [ ] `ApiError` and `ApiErrorCode` match the error codes documented in `API.md`.
- [ ] Unit tests cover representative model construction and serde trait availability.
- [ ] `cargo fmt --check` passes.
- [ ] `cargo check --workspace` passes.
- [ ] `cargo test --workspace` passes.
- [ ] No proxy, protocol parser, storage, API handler, or frontend logic is introduced.

## Open Questions

None blocking.
