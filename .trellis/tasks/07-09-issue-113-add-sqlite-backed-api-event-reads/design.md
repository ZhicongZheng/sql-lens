# Issue 113 Design: Configured API Event Read Source

## Scope

Add a backend read path that lets API consumers read SQL event timeline/detail
data from SQLite when the app runtime is configured with `storage.type =
"sqlite"`.

## Approach

Introduce a small `sql-lens-api` read-source boundary for SQL events:

- The default read source wraps the existing `RingBufferStore`.
- A SQLite read source wraps a thread-safe `SqliteEventStore`.
- Handlers call the read-source boundary instead of directly acquiring
  `state.event_store()` for list/detail/export/replay preview event lookup.
- `sql-lens-app` creates the SQLite read source from the same configured
  SQLite path used for persistence.

This keeps the REST behavior selected by runtime configuration without turning
`sql-lens-storage` into a generic async repository layer.

## Data Flow

```text
storage.type = "ring_buffer"
  -> ApiState event reader = ring buffer
  -> REST list/detail/export/replay reads live memory

storage.type = "sqlite"
  -> app opens SQLite write worker for persistence
  -> app opens SQLite read store for ApiState
  -> REST list/detail/export/replay reads persisted rows
```

## Contracts

- Existing REST response DTOs remain unchanged.
- Existing query parameters remain unchanged.
- Ring buffer cursor format stays `seq_<number>`.
- SQLite cursor format should be distinct from ring buffer cursors so clients do
  not accidentally mix cursor families.
- API errors continue using `ApiEndpointError` and request ID middleware.
- SQLite read source must include parameter rows for detail responses.

## Storage Mapping

SQLite stores rows, parameters, and protocol metadata JSON separately from the
core `SqlEvent` model. The API read boundary can map SQLite rows directly into
existing response DTOs to avoid lossy core reconstruction.

Required mappings:

- `SqliteEventRow` -> `SqlEventSummaryResponse`
- `SqliteEventRow + Vec<SqliteParameterRow>` -> `SqlEventDetailResponse`
- SQLite timeline cursor -> API `next_cursor`

## Error Handling

SQLite query errors map to API envelopes:

- Invalid filters -> `BAD_REQUEST`.
- SQLite read failures -> `STORAGE_UNAVAILABLE` or `INTERNAL`, depending on the
  existing API error constructors available after implementation review.

If a new API error constructor is needed, keep it structured and tested.

## Compatibility

- Ring-buffer-only tests should keep passing unchanged.
- SQLite mode changes only configured runtime API reads.
- WebSocket live updates and live statistics stay memory based.
- SQLite writes remain on the existing worker thread and must not block proxy
  forwarding.
