# SQL Lens GitHub Issues

This backlog is organized as copy-ready GitHub issue drafts. Each issue is intended to be small enough for an independent pull request.

## Issue 001: Create Rust workspace skeleton

Description: Create the initial Rust workspace layout with the planned SQL Lens crates and placeholder library files only.

Acceptance Criteria:

- Workspace contains the planned crate directories.
- Each crate has a minimal compilable entry point.
- No business logic is introduced.

Labels: `area:backend`, `area:infrastructure`, `type:task`
Priority: P0
Difficulty: Easy
Estimated Time: 2h
Dependencies: None

## Issue 002: Add core domain crate

Description: Add `sql-lens-core` as the owner of protocol-neutral capture models.

Acceptance Criteria:

- Crate exists and builds.
- Public module placeholders match documented domains.
- Crate has no protocol-specific dependencies.

Labels: `area:core`, `type:task`
Priority: P0
Difficulty: Easy
Estimated Time: 2h
Dependencies: Issue 001

## Issue 003: Define initial SqlEvent model

Description: Define the first `SqlEvent` model with protocol-neutral fields and metadata extension support.

Acceptance Criteria:

- Model includes IDs, timestamps, protocol, connection, SQL, status, timing, result, error, and metadata fields.
- Serialization tests cover a representative event.
- No MySQL-only fields exist outside metadata.

Labels: `area:core`, `type:feature`, `contract:api`
Priority: P0
Difficulty: Medium
Estimated Time: 4h
Dependencies: Issue 002

## Issue 004: Define connection domain model

Description: Define `ConnectionInfo` and connection state enums.

Acceptance Criteria:

- Model includes client, backend, protocol, database, user, state, timestamps, bytes, and query count.
- Serialization tests cover active and closed states.
- State names match architecture docs.

Labels: `area:core`, `type:feature`
Priority: P0
Difficulty: Easy
Estimated Time: 3h
Dependencies: Issue 002

## Issue 005: Define SQL parameter model

Description: Define protocol-neutral SQL parameter types for prepared statement expansion.

Acceptance Criteria:

- Model supports null, numeric, boolean, string, date/time, JSON, and binary summary.
- Redaction state is represented.
- Tests cover serialization for each parameter class.

Labels: `area:core`, `type:feature`
Priority: P0
Difficulty: Medium
Estimated Time: 4h
Dependencies: Issue 002

## Issue 006: Define API error model

Description: Add shared API error codes and error response types.

Acceptance Criteria:

- Error codes match `API.md`.
- Error response includes code, message, request ID, and details.
- Serialization tests are included.

Labels: `area:api`, `area:core`, `type:feature`
Priority: P1
Difficulty: Easy
Estimated Time: 3h
Dependencies: Issue 002

## Issue 007: Add configuration crate module

Description: Add configuration model scaffolding for proxy, backend, TLS, web, storage, retention, logging, redaction, auth, replay, and plugins.

Acceptance Criteria:

- Config structs match `CONFIG.md`.
- Defaults are explicit.
- Unit tests cover default config creation.

Labels: `area:config`, `type:feature`
Priority: P0
Difficulty: Medium
Estimated Time: 5h
Dependencies: Issue 001

## Issue 008: Implement TOML config loading

Description: Load SQL Lens configuration from a TOML file.

Acceptance Criteria:

- Config can be loaded from a path.
- Invalid TOML returns a structured error.
- Tests cover valid and invalid config files.

Labels: `area:config`, `type:feature`
Priority: P0
Difficulty: Medium
Estimated Time: 4h
Dependencies: Issue 007

## Issue 009: Implement environment variable overrides

Description: Support `SQL_LENS_` environment variable overrides for key config fields.

Acceptance Criteria:

- Proxy listen, backend address, and log level can be overridden.
- Override precedence is documented in tests.
- Invalid overrides return useful errors.

Labels: `area:config`, `type:feature`
Priority: P1
Difficulty: Medium
Estimated Time: 4h
Dependencies: Issue 008

## Issue 010: Add config validation

Description: Validate required config fields before services start.

Acceptance Criteria:

- Missing proxy listen is rejected.
- Missing backend address is rejected in proxy mode.
- Unknown protocol adapter is rejected.

Labels: `area:config`, `type:feature`
Priority: P0
Difficulty: Easy
Estimated Time: 3h
Dependencies: Issue 008

## Issue 011: Add CLI entry point

Description: Add the initial `sql-lens` binary entry point with config path argument.

Acceptance Criteria:

- Binary accepts `--config`.
- Binary prints version.
- Startup errors are displayed clearly.

Labels: `area:cli`, `type:feature`
Priority: P0
Difficulty: Easy
Estimated Time: 3h
Dependencies: Issue 008

## Issue 012: Initialize structured logging

Description: Set up structured logging with configurable level and format.

Acceptance Criteria:

- JSON and pretty formats are supported.
- Log level comes from config.
- Tests or smoke checks verify initialization.

Labels: `area:observability`, `type:task`
Priority: P0
Difficulty: Easy
Estimated Time: 3h
Dependencies: Issue 007

## Issue 013: Add TCP proxy listener

Description: Implement the TCP listener that accepts client database connections.

Acceptance Criteria:

- Listener binds to configured address.
- Bind failures return structured errors.
- Connection accept loop can be shut down.

Labels: `area:proxy`, `type:feature`
Priority: P0
Difficulty: Medium
Estimated Time: 5h
Dependencies: Issue 011

## Issue 014: Implement backend dialing

Description: Connect accepted client sessions to the configured backend database.

Acceptance Criteria:

- Backend address is read from config.
- Dial timeout is enforced.
- Dial failures create connection failure records.

Labels: `area:proxy`, `type:feature`
Priority: P0
Difficulty: Medium
Estimated Time: 5h
Dependencies: Issue 013

## Issue 015: Implement bidirectional TCP forwarding

Description: Forward bytes between client and backend connections.

Acceptance Criteria:

- Client-to-backend and backend-to-client copy loops work.
- Byte counters are updated.
- Either side closing shuts down the session cleanly.

Labels: `area:proxy`, `type:feature`
Priority: P0
Difficulty: Medium
Estimated Time: 6h
Dependencies: Issue 014

## Issue 016: Add proxy graceful shutdown

Description: Stop accepting new connections and drain active sessions during shutdown.

Acceptance Criteria:

- Shutdown signal stops listener.
- Active sessions receive shutdown notification.
- Shutdown timeout is configurable.

Labels: `area:proxy`, `type:feature`
Priority: P1
Difficulty: Medium
Estimated Time: 5h
Dependencies: Issue 015

## Issue 017: Track connection lifecycle

Description: Record connection lifecycle states from accept to close.

Acceptance Criteria:

- Connection ID is generated for each session.
- State transitions match `ARCHITECTURE.md`.
- Unit tests cover normal close and backend dial failure.

Labels: `area:proxy`, `area:core`, `type:feature`
Priority: P0
Difficulty: Medium
Estimated Time: 5h
Dependencies: Issue 004, Issue 014

## Issue 018: Add capture pipeline channel

Description: Add a bounded channel for capture events from proxy/protocol logic to storage and broadcast consumers.

Acceptance Criteria:

- Channel capacity is configurable.
- Overload policy is explicit.
- Dropped-event counter exists.

Labels: `area:capture`, `type:feature`
Priority: P0
Difficulty: Medium
Estimated Time: 5h
Dependencies: Issue 003

## Issue 019: Define protocol adapter trait

Description: Add the shared trait for protocol adapters.

Acceptance Criteria:

- Trait can observe client and backend bytes.
- Trait can emit capture events.
- Trait supports protocol-specific connection state.

Labels: `area:protocol`, `type:feature`, `contract:internal`
Priority: P0
Difficulty: Medium
Estimated Time: 5h
Dependencies: Issue 003, Issue 004

## Issue 020: Add protocol adapter registry

Description: Add a registry that selects protocol adapters by configured protocol name.

Acceptance Criteria:

- Registry can register and resolve adapters.
- Unknown adapter names produce config validation errors.
- Tests cover adapter lookup.

Labels: `area:protocol`, `area:config`, `type:feature`
Priority: P0
Difficulty: Easy
Estimated Time: 3h
Dependencies: Issue 019

## Issue 021: Implement in-memory ring buffer append

Description: Implement append-only storage for SQL events in a fixed-size ring buffer.

Acceptance Criteria:

- Events can be appended.
- Capacity is enforced.
- Oldest events are evicted by default.

Labels: `area:storage`, `type:feature`
Priority: P0
Difficulty: Medium
Estimated Time: 5h
Dependencies: Issue 003

## Issue 022: Implement ring buffer event lookup

Description: Support lookup of captured SQL events by ID.

Acceptance Criteria:

- Existing events can be retrieved.
- Evicted events return not found.
- Tests cover both cases.

Labels: `area:storage`, `type:feature`
Priority: P0
Difficulty: Easy
Estimated Time: 3h
Dependencies: Issue 021

## Issue 023: Implement ring buffer timeline query

Description: Query recent events in reverse chronological order.

Acceptance Criteria:

- Query supports limit.
- Query returns stable cursors or documented cursor placeholder.
- Tests cover ordering.

Labels: `area:storage`, `type:feature`
Priority: P0
Difficulty: Medium
Estimated Time: 4h
Dependencies: Issue 021

## Issue 024: Implement storage filters

Description: Add filters for protocol, database type, database, user, status, duration, text, and time range.

Acceptance Criteria:

- Filters can be combined.
- Tests cover at least five filter combinations.
- Unsupported filters return clear errors.

Labels: `area:storage`, `type:feature`
Priority: P1
Difficulty: Medium
Estimated Time: 6h
Dependencies: Issue 023

## Issue 025: Add live statistics counters

Description: Maintain lightweight counters for dashboard metrics.

Acceptance Criteria:

- Tracks QPS, errors, slow SQL, latency buckets, and active connections.
- Counters update from capture events.
- Tests cover counter updates.

Labels: `area:statistics`, `type:feature`
Priority: P1
Difficulty: Medium
Estimated Time: 5h
Dependencies: Issue 018, Issue 021

## Issue 026: Add HTTP server foundation

Description: Start the web/API HTTP server on the configured web address.

Acceptance Criteria:

- Server binds to `web.listen`.
- Server shuts down gracefully.
- Request IDs are attached to requests.

Labels: `area:api`, `type:feature`
Priority: P0
Difficulty: Medium
Estimated Time: 5h
Dependencies: Issue 011

## Issue 027: Implement health endpoint

Description: Add `GET /api/v1/health`.

Acceptance Criteria:

- Endpoint returns status, version, and uptime.
- Endpoint works without storage data.
- Test covers response schema.

Labels: `area:api`, `type:feature`
Priority: P0
Difficulty: Easy
Estimated Time: 2h
Dependencies: Issue 026

## Issue 028: Implement SQL event list endpoint

Description: Add `GET /api/v1/sql-events`.

Acceptance Criteria:

- Endpoint reads from storage.
- Query parameters map to storage filters.
- Response matches `API.md`.

Labels: `area:api`, `type:feature`
Priority: P0
Difficulty: Medium
Estimated Time: 5h
Dependencies: Issue 024, Issue 026

## Issue 029: Implement SQL event detail endpoint

Description: Add `GET /api/v1/sql-events/{id}`.

Acceptance Criteria:

- Existing event returns detail.
- Missing event returns `NOT_FOUND`.
- Response includes parameters and metadata.

Labels: `area:api`, `type:feature`
Priority: P0
Difficulty: Easy
Estimated Time: 3h
Dependencies: Issue 022, Issue 026

## Issue 030: Implement connections endpoint

Description: Add list and detail endpoints for connection state.

Acceptance Criteria:

- `GET /api/v1/connections` returns recent connections.
- `GET /api/v1/connections/{id}` returns detail.
- Tests cover active and closed connections.

Labels: `area:api`, `area:proxy`, `type:feature`
Priority: P0
Difficulty: Medium
Estimated Time: 5h
Dependencies: Issue 017, Issue 026

## Issue 031: Implement statistics endpoint

Description: Add `GET /api/v1/statistics`.

Acceptance Criteria:

- Endpoint returns QPS, error rate, slow count, latency percentiles, and active connections.
- Window parameter is validated.
- Tests cover empty and populated state.

Labels: `area:api`, `area:statistics`, `type:feature`
Priority: P1
Difficulty: Medium
Estimated Time: 4h
Dependencies: Issue 025, Issue 026

## Issue 032: Implement protocols endpoint

Description: Add `GET /api/v1/protocols`.

Acceptance Criteria:

- Endpoint lists supported and planned protocol families.
- MySQL is marked supported when adapter is enabled.
- Response is protocol-neutral.

Labels: `area:api`, `area:protocol`, `type:feature`
Priority: P2
Difficulty: Easy
Estimated Time: 2h
Dependencies: Issue 020, Issue 026

## Issue 033: Standardize REST error responses

Description: Ensure all REST errors use the documented `ApiError` shape.

Acceptance Criteria:

- 400, 401, 403, 404, 409, 429, 500, and 503 mappings exist.
- Request ID is included.
- Tests cover representative errors.

Labels: `area:api`, `type:task`, `contract:api`
Priority: P0
Difficulty: Medium
Estimated Time: 4h
Dependencies: Issue 006, Issue 026

## Issue 034: Add WebSocket server foundation

Description: Add WebSocket upgrade support for live SQL streams.

Acceptance Criteria:

- `/ws/sql` accepts connections.
- Server handles disconnects cleanly.
- Basic ping/pong or heartbeat exists.

Labels: `area:websocket`, `type:feature`
Priority: P1
Difficulty: Medium
Estimated Time: 5h
Dependencies: Issue 026

## Issue 035: Implement SQL WebSocket subscription

Description: Broadcast new SQL events to WebSocket subscribers.

Acceptance Criteria:

- Subscribers receive `sql_event.created`.
- Event payload includes type, version, and payload.
- Tests or integration smoke check cover one subscriber.

Labels: `area:websocket`, `area:capture`, `type:feature`
Priority: P1
Difficulty: Medium
Estimated Time: 5h
Dependencies: Issue 018, Issue 034

## Issue 036: Add WebSocket filters

Description: Let clients subscribe to filtered SQL event streams.

Acceptance Criteria:

- Filters support protocol, status, database, and duration.
- Invalid filters return a subscription error.
- Tests cover matching and non-matching events.

Labels: `area:websocket`, `type:feature`
Priority: P2
Difficulty: Medium
Estimated Time: 4h
Dependencies: Issue 035

## Issue 037: Add MySQL protocol crate

Description: Create `sql-lens-protocol-mysql` crate and wire it into the adapter registry.

Acceptance Criteria:

- Crate builds.
- Adapter can be registered as `mysql`.
- No parsing behavior is required yet.

Labels: `area:protocol-mysql`, `type:task`
Priority: P0
Difficulty: Easy
Estimated Time: 2h
Dependencies: Issue 019, Issue 020

## Issue 038: Parse MySQL packet header

Description: Implement MySQL-compatible packet envelope parsing.

Acceptance Criteria:

- Parses 3-byte payload length.
- Parses 1-byte sequence ID.
- Rejects incomplete packets gracefully.

Labels: `area:protocol-mysql`, `type:feature`
Priority: P0
Difficulty: Medium
Estimated Time: 4h
Dependencies: Issue 037

## Issue 039: Add MySQL packet fixture tests

Description: Add golden packet fixtures for MySQL packet framing.

Acceptance Criteria:

- Fixtures include normal, empty, and malformed packets.
- Tests assert parsed length and sequence ID.
- Fixture format is documented.

Labels: `area:protocol-mysql`, `area:testing`, `type:test`
Priority: P0
Difficulty: Easy
Estimated Time: 3h
Dependencies: Issue 038

## Issue 040: Observe MySQL initial handshake

Description: Decode enough of the server initial handshake to identify protocol setup.

Acceptance Criteria:

- Adapter detects server handshake packet.
- Connection state moves to handshake seen.
- Sensitive data is not logged.

Labels: `area:protocol-mysql`, `type:feature`
Priority: P0
Difficulty: Medium
Estimated Time: 5h
Dependencies: Issue 038

## Issue 041: Observe MySQL client handshake response

Description: Observe client authentication response without storing secrets.

Acceptance Criteria:

- Adapter detects client handshake response.
- Username and requested database are captured when safe.
- Authentication response bytes are not persisted.

Labels: `area:protocol-mysql`, `area:security`, `type:feature`
Priority: P0
Difficulty: Medium
Estimated Time: 5h
Dependencies: Issue 040

## Issue 042: Detect MySQL authentication result

Description: Detect whether authentication succeeds or fails.

Acceptance Criteria:

- OK packet marks connection authenticated.
- Error packet marks auth failed.
- Connection state is updated.

Labels: `area:protocol-mysql`, `type:feature`
Priority: P0
Difficulty: Medium
Estimated Time: 4h
Dependencies: Issue 041

## Issue 043: Parse COM_QUERY

Description: Parse MySQL `COM_QUERY` command payloads.

Acceptance Criteria:

- SQL text is extracted.
- Command type is recorded.
- Invalid UTF-8 is handled safely.

Labels: `area:protocol-mysql`, `type:feature`
Priority: P0
Difficulty: Medium
Estimated Time: 5h
Dependencies: Issue 038

## Issue 044: Capture COM_QUERY timing

Description: Measure duration from `COM_QUERY` to backend response completion.

Acceptance Criteria:

- Event duration is recorded.
- OK and error responses finalize the event.
- Tests cover success and error paths.

Labels: `area:protocol-mysql`, `area:capture`, `type:feature`
Priority: P0
Difficulty: Medium
Estimated Time: 6h
Dependencies: Issue 043

## Issue 045: Parse MySQL OK packet summary

Description: Decode basic OK packet fields for affected rows and status.

Acceptance Criteria:

- Affected rows are captured when available.
- Status is mapped to success.
- Tests cover a fixture OK packet.

Labels: `area:protocol-mysql`, `type:feature`
Priority: P1
Difficulty: Medium
Estimated Time: 4h
Dependencies: Issue 044

## Issue 046: Parse MySQL error packet summary

Description: Decode MySQL error packets into protocol-neutral error summaries.

Acceptance Criteria:

- Error code is captured.
- SQLSTATE is captured when present.
- Error message is sanitized.

Labels: `area:protocol-mysql`, `area:security`, `type:feature`
Priority: P0
Difficulty: Medium
Estimated Time: 4h
Dependencies: Issue 044

## Issue 047: Parse COM_STMT_PREPARE

Description: Parse prepared statement prepare commands.

Acceptance Criteria:

- SQL template is extracted.
- Prepare command creates a pending statement record.
- Tests cover prepare command fixture.

Labels: `area:protocol-mysql`, `type:feature`
Priority: P0
Difficulty: Medium
Estimated Time: 5h
Dependencies: Issue 038

## Issue 048: Parse COM_STMT_PREPARE response

Description: Parse backend prepare response to obtain statement ID and counts.

Acceptance Criteria:

- Statement ID is captured.
- Parameter and column counts are captured.
- Prepare errors are handled.

Labels: `area:protocol-mysql`, `type:feature`
Priority: P0
Difficulty: Hard
Estimated Time: 7h
Dependencies: Issue 047

## Issue 049: Store prepared statement state per connection

Description: Maintain statement ID to SQL template mappings scoped to one connection.

Acceptance Criteria:

- Statement state is connection-local.
- Statement state is removed on connection close.
- Tests prove no cross-connection leakage.

Labels: `area:protocol-mysql`, `area:core`, `type:feature`
Priority: P0
Difficulty: Medium
Estimated Time: 5h
Dependencies: Issue 048

## Issue 050: Parse COM_STMT_EXECUTE envelope

Description: Parse statement execute command header, statement ID, flags, iteration count, and parameter metadata marker.

Acceptance Criteria:

- Statement ID is extracted.
- Unknown statement IDs are reported gracefully.
- Tests cover fixture command.

Labels: `area:protocol-mysql`, `type:feature`
Priority: P0
Difficulty: Hard
Estimated Time: 7h
Dependencies: Issue 049

## Issue 051: Decode MySQL NULL bitmap

Description: Decode the NULL bitmap used by MySQL prepared statement execute.

Acceptance Criteria:

- NULL parameter positions are identified.
- Tests cover mixed NULL and non-NULL parameters.
- Malformed bitmap returns a structured parse error.

Labels: `area:protocol-mysql`, `type:feature`
Priority: P0
Difficulty: Medium
Estimated Time: 4h
Dependencies: Issue 050

## Issue 052: Decode common MySQL numeric parameters

Description: Decode integer and floating point prepared statement parameters.

Acceptance Criteria:

- Signed and unsigned integer classes are decoded.
- Float and double are decoded.
- Tests cover representative values.

Labels: `area:protocol-mysql`, `type:feature`
Priority: P0
Difficulty: Hard
Estimated Time: 7h
Dependencies: Issue 050, Issue 051

## Issue 053: Decode common MySQL string and binary parameters

Description: Decode string-like parameters and summarize binary values.

Acceptance Criteria:

- Text values are decoded safely.
- Binary values use summaries by default.
- Invalid text is represented without panics.

Labels: `area:protocol-mysql`, `area:security`, `type:feature`
Priority: P0
Difficulty: Hard
Estimated Time: 7h
Dependencies: Issue 050, Issue 051

## Issue 054: Decode MySQL date and time parameters

Description: Decode date, time, datetime, and timestamp parameter values.

Acceptance Criteria:

- Common date/time formats are decoded.
- Edge cases are represented clearly.
- Tests cover date, time, and timestamp.

Labels: `area:protocol-mysql`, `type:feature`
Priority: P1
Difficulty: Hard
Estimated Time: 6h
Dependencies: Issue 050, Issue 051

## Issue 055: Render expanded SQL for prepared statements

Description: Render readable SQL by replacing placeholders with escaped parameter literals.

Acceptance Criteria:

- Strings are quoted and escaped.
- NULL renders as `NULL`.
- Binary values render as summaries.
- Rendering never modifies forwarded traffic.

Labels: `area:core`, `area:protocol-mysql`, `type:feature`
Priority: P0
Difficulty: Hard
Estimated Time: 8h
Dependencies: Issue 005, Issue 052, Issue 053

## Issue 056: Apply redaction before storage

Description: Ensure sensitive parameters and SQL text are redacted before events reach storage or WebSocket.

Acceptance Criteria:

- Redaction rules apply to parameters.
- Redaction rules apply to expanded SQL.
- Tests prove storage receives redacted values.

Labels: `area:security`, `area:storage`, `type:feature`
Priority: P0
Difficulty: Hard
Estimated Time: 7h
Dependencies: Issue 055

## Issue 057: Implement COM_STMT_CLOSE cleanup

Description: Parse close commands and remove statement state.

Acceptance Criteria:

- Statement ID is removed from connection state.
- Closing unknown statement is harmless.
- Tests cover cleanup.

Labels: `area:protocol-mysql`, `type:feature`
Priority: P1
Difficulty: Medium
Estimated Time: 4h
Dependencies: Issue 049

## Issue 058: Observe COM_PING and COM_QUIT

Description: Track ping and quit commands as connection activity.

Acceptance Criteria:

- Ping updates last activity.
- Quit moves connection toward closing.
- Ping is not stored as SQL by default.

Labels: `area:protocol-mysql`, `area:proxy`, `type:feature`
Priority: P1
Difficulty: Easy
Estimated Time: 3h
Dependencies: Issue 042

## Issue 059: Add MySQL live integration test with Docker

Description: Add a Docker-based integration test for MySQL through SQL Lens.

Acceptance Criteria:

- Test starts MySQL and SQL Lens.
- Test runs a simple query through the proxy.
- API shows the captured SQL event.

Labels: `area:testing`, `area:protocol-mysql`, `type:test`
Priority: P0
Difficulty: Hard
Estimated Time: 8h
Dependencies: Issue 028, Issue 044

## Issue 060: Add prepared statement integration test

Description: Add an integration test proving prepared statement capture and expansion with a real driver.

Acceptance Criteria:

- Test prepares and executes a parameterized query.
- API returns original SQL, parameters, and expanded SQL.
- Redaction behavior is covered for one parameter.

Labels: `area:testing`, `area:protocol-mysql`, `type:test`
Priority: P0
Difficulty: Hard
Estimated Time: 8h
Dependencies: Issue 055, Issue 059

## Issue 061: Add StarRocks compatibility smoke test

Description: Add a compatibility smoke test for StarRocks when running the extended matrix.

Acceptance Criteria:

- Test can run separately from default CI.
- Basic connect and query path is covered.
- Known compatibility gaps are documented.

Labels: `area:compatibility`, `type:test`
Priority: P2
Difficulty: Hard
Estimated Time: 8h
Dependencies: Issue 059

## Issue 062: Add TiDB compatibility smoke test

Description: Add a compatibility smoke test for TiDB.

Acceptance Criteria:

- Test runs connect, text query, and prepared query.
- Results are compared to MySQL behavior.
- Gaps are documented.

Labels: `area:compatibility`, `type:test`
Priority: P2
Difficulty: Hard
Estimated Time: 8h
Dependencies: Issue 060

## Issue 063: Add Doris compatibility smoke test

Description: Add a compatibility smoke test for Apache Doris.

Acceptance Criteria:

- Test runs connect and text query.
- Prepared statement behavior is documented.
- Gaps are tracked as follow-up issues.

Labels: `area:compatibility`, `type:test`
Priority: P2
Difficulty: Hard
Estimated Time: 8h
Dependencies: Issue 059

## Issue 064: Create React web app skeleton

Description: Create the initial React TypeScript web app structure.

Acceptance Criteria:

- App builds.
- Directory layout matches `UI.md`.
- No backend coupling is hardcoded.

Labels: `area:frontend`, `type:task`
Priority: P0
Difficulty: Medium
Estimated Time: 5h
Dependencies: Issue 001

## Issue 065: Add shadcn/ui and Tailwind foundation

Description: Set up TailwindCSS and shadcn/ui base components.

Acceptance Criteria:

- Tailwind config exists.
- shadcn/ui components can be generated or imported.
- Light and dark theme tokens exist.

Labels: `area:frontend`, `type:task`
Priority: P0
Difficulty: Medium
Estimated Time: 5h
Dependencies: Issue 064

## Issue 066: Add frontend API client

Description: Add typed API client functions for SQL events, connections, statistics, and protocols.

Acceptance Criteria:

- Client functions are typed.
- Errors map to API error model.
- Tests cover one successful and one failed request.

Labels: `area:frontend`, `area:api`, `type:feature`
Priority: P0
Difficulty: Medium
Estimated Time: 5h
Dependencies: Issue 028, Issue 064

## Issue 108: Add frontend multi-target proxy support

Description: Adapt the frontend for SQL Lens installations with multiple configured proxy targets, such as MySQL and StarRocks listeners running in one process.

Acceptance Criteria:

- API client and UI types can represent target identity when the backend exposes it.
- SQL event lists and details show which configured target captured the event.
- Filters can narrow SQL events by target without replacing protocol or database type filters.
- Empty, loading, and error states handle deployments with zero, one, or many targets.

Labels: `area:frontend`, `area:api`, `type:feature`
Priority: P1
Difficulty: Medium
Estimated Time: 5h
Dependencies: Issue 066, Add multi-target proxy configuration and runtime fan-out

## Issue 067: Add TanStack Query providers

Description: Configure TanStack Query for server state.

Acceptance Criteria:

- Query client provider exists.
- Default retry and stale-time policy is documented.
- SQL events query uses the API client.

Labels: `area:frontend`, `type:task`
Priority: P0
Difficulty: Easy
Estimated Time: 3h
Dependencies: Issue 066

## Issue 068: Build app layout shell

Description: Build the main navigation, top bar, and content layout.

Acceptance Criteria:

- Navigation includes Dashboard, SQL, Connections, Statistics, Replay, Settings.
- Layout supports dark mode.
- Mobile layout does not break.

Labels: `area:frontend`, `type:feature`
Priority: P0
Difficulty: Medium
Estimated Time: 5h
Dependencies: Issue 065

## Issue 069: Build Dashboard page

Description: Implement the initial Dashboard using API statistics.

Acceptance Criteria:

- Displays QPS, latency, active connections, slow count, and error count.
- Empty state is handled.
- Loading and error states are implemented.

Labels: `area:frontend`, `area:statistics`, `type:feature`
Priority: P1
Difficulty: Medium
Estimated Time: 6h
Dependencies: Issue 031, Issue 068

## Issue 070: Build SQL List page

Description: Implement the primary SQL timeline table.

Acceptance Criteria:

- Table shows columns documented in `UI.md`.
- Pagination or cursor loading works.
- Loading, empty, and error states exist.

Labels: `area:frontend`, `type:feature`
Priority: P0
Difficulty: Medium
Estimated Time: 6h
Dependencies: Issue 028, Issue 068

## Issue 071: Add SQL List filters

Description: Add filters for text, protocol, status, database, user, and duration.

Acceptance Criteria:

- Filters update API query parameters.
- Filter state is reflected in the URL where useful.
- Clear filters action exists.

Labels: `area:frontend`, `type:feature`
Priority: P1
Difficulty: Medium
Estimated Time: 5h
Dependencies: Issue 070

## Issue 072: Add SQL WebSocket client

Description: Add frontend WebSocket client for live SQL events.

Acceptance Criteria:

- Client connects to `/ws/sql`.
- Incoming events update SQL List cache.
- Disconnect state is visible.

Labels: `area:frontend`, `area:websocket`, `type:feature`
Priority: P1
Difficulty: Medium
Estimated Time: 5h
Dependencies: Issue 035, Issue 070

## Issue 073: Add pause live updates control

Description: Let users pause and resume live SQL List updates.

Acceptance Criteria:

- Paused mode stops auto-inserting visible events.
- Resume applies queued or refreshed events predictably.
- State is visible in the UI.

Labels: `area:frontend`, `type:feature`
Priority: P2
Difficulty: Medium
Estimated Time: 4h
Dependencies: Issue 072

## Issue 074: Build SQL Detail page

Description: Implement SQL event detail view.

Acceptance Criteria:

- Shows summary, original SQL, expanded SQL, parameters, timings, result, error, connection, and metadata.
- Missing event shows not found state.
- Copy actions are available for SQL text.

Labels: `area:frontend`, `type:feature`
Priority: P0
Difficulty: Medium
Estimated Time: 7h
Dependencies: Issue 029, Issue 070

## Issue 075: Integrate Monaco SQL viewer

Description: Add read-only Monaco Editor for SQL display.

Acceptance Criteria:

- Original and expanded SQL can be toggled.
- SQL is rendered as text.
- Theme follows light/dark mode.

Labels: `area:frontend`, `type:feature`
Priority: P1
Difficulty: Medium
Estimated Time: 5h
Dependencies: Issue 074

## Issue 076: Build parameter table component

Description: Display SQL parameters with type, value, index, and redaction state.

Acceptance Criteria:

- Redacted values are clearly marked.
- Binary summaries are displayed safely.
- Long values are truncated with expansion.

Labels: `area:frontend`, `area:security`, `type:feature`
Priority: P1
Difficulty: Medium
Estimated Time: 5h
Dependencies: Issue 074

## Issue 077: Build Connections page

Description: Implement active and recent connections view.

Acceptance Criteria:

- Connection table shows documented fields.
- Users can filter active and closed connections.
- Row opens connection detail.

Labels: `area:frontend`, `area:proxy`, `type:feature`
Priority: P1
Difficulty: Medium
Estimated Time: 5h
Dependencies: Issue 030, Issue 068

## Issue 078: Build Statistics page

Description: Implement charts for query volume, latency, errors, and top fingerprints.

Acceptance Criteria:

- Uses ECharts.
- Handles empty state.
- Time window selector updates data.

Labels: `area:frontend`, `area:statistics`, `type:feature`
Priority: P2
Difficulty: Medium
Estimated Time: 6h
Dependencies: Issue 031, Issue 068

## Issue 079: Build Settings page skeleton

Description: Add settings sections for proxy, backend, storage, redaction, auth, plugins, and exporters.

Acceptance Criteria:

- Sections match `UI.md`.
- Read-only placeholder is acceptable for v1 skeleton.
- Restart-required fields are visually marked.

Labels: `area:frontend`, `type:feature`
Priority: P2
Difficulty: Medium
Estimated Time: 5h
Dependencies: Issue 068

## Issue 080: Add replay preview API

Description: Implement replay preview endpoint that renders SQL and risk classification without executing it.

Acceptance Criteria:

- Endpoint accepts event ID or SQL payload.
- Response includes final SQL and mutation warning.
- No SQL is executed.

Labels: `area:api`, `area:replay`, `type:feature`
Priority: P1
Difficulty: Medium
Estimated Time: 5h
Dependencies: Issue 029, Issue 055

## Issue 081: Add replay UI preview

Description: Build the replay preview UI with explicit safety messaging.

Acceptance Criteria:

- Shows target, SQL, and risk classification.
- Mutating SQL is clearly warned.
- Execute button can remain disabled until execution endpoint exists.

Labels: `area:frontend`, `area:replay`, `type:feature`
Priority: P2
Difficulty: Medium
Estimated Time: 5h
Dependencies: Issue 080

## Issue 082: Add slow SQL classification

Description: Classify SQL events as slow based on configured thresholds.

Acceptance Criteria:

- Global threshold is supported.
- Slow status appears in stored events.
- Tests cover below and above threshold.

Labels: `area:capture`, `area:statistics`, `type:feature`
Priority: P0
Difficulty: Medium
Estimated Time: 4h
Dependencies: Issue 003, Issue 007

## Issue 083: Add error SQL classification

Description: Classify failed SQL events as error events.

Acceptance Criteria:

- Error packet summaries set status to error.
- Error counters are updated.
- API filters can return only errors.

Labels: `area:capture`, `area:protocol-mysql`, `type:feature`
Priority: P0
Difficulty: Medium
Estimated Time: 4h
Dependencies: Issue 046

## Issue 084: Implement SQL fingerprinting foundation

Description: Add a simple SQL fingerprint function for grouping similar queries.

Acceptance Criteria:

- Literal values are normalized.
- Basic whitespace normalization exists.
- Tests cover common SELECT, INSERT, UPDATE, DELETE.

Labels: `area:core`, `type:feature`
Priority: P2
Difficulty: Medium
Estimated Time: 5h
Dependencies: Issue 003

## Issue 085: Add SQL export endpoint

Description: Export filtered SQL events as JSON or NDJSON.

Acceptance Criteria:

- Export respects current filters.
- Exported events are redacted.
- Large exports are bounded.

Labels: `area:api`, `type:feature`
Priority: P2
Difficulty: Medium
Estimated Time: 5h
Dependencies: Issue 028, Issue 056

## Issue 086: Add SQLite storage schema design

Description: Implement the first SQLite schema and migration table.

Acceptance Criteria:

- Tables match `STORAGE.md`.
- Schema version table exists.
- Migration can be applied to an empty database.

Labels: `area:storage`, `type:feature`
Priority: P2
Difficulty: Medium
Estimated Time: 6h
Dependencies: Issue 021

## Issue 087: Implement SQLite event inserts

Description: Persist captured SQL events into SQLite when configured.

Acceptance Criteria:

- Inserts are asynchronous or buffered.
- Redacted events are stored.
- Tests cover insert and readback.

Labels: `area:storage`, `type:feature`
Priority: P2
Difficulty: Hard
Estimated Time: 8h
Dependencies: Issue 086

## Issue 088: Implement SQLite timeline queries

Description: Query persisted SQL events from SQLite with filters.

Acceptance Criteria:

- Timeline query matches ring buffer behavior.
- Common filters use indexes.
- Tests cover pagination.

Labels: `area:storage`, `type:feature`
Priority: P2
Difficulty: Hard
Estimated Time: 8h
Dependencies: Issue 087

## Issue 089: Add retention policy enforcement

Description: Enforce max age, max events, and max bytes retention policies where supported.

Acceptance Criteria:

- Ring buffer respects max events.
- SQLite supports age and event-count cleanup.
- Tests cover cleanup behavior.

Labels: `area:storage`, `type:feature`
Priority: P2
Difficulty: Medium
Estimated Time: 6h
Dependencies: Issue 021, Issue 087

## Issue 090: Add plugin hook trait definitions

Description: Define plugin hook traits for connect, query, prepare, execute, and error events.

Acceptance Criteria:

- Hook names match `PLUGIN.md`.
- Hook payloads use protocol-neutral types.
- Plugin errors are isolated.

Labels: `area:plugin`, `type:feature`
Priority: P2
Difficulty: Medium
Estimated Time: 5h
Dependencies: Issue 003, Issue 004

## Issue 091: Add webhook exporter skeleton

Description: Add an exporter skeleton for sending redacted SQL events to webhooks.

Acceptance Criteria:

- Exporter accepts redacted events.
- Timeout is configurable.
- Signature header design is documented in code comments or docs.

Labels: `area:plugin`, `area:exporter`, `type:feature`
Priority: P3
Difficulty: Medium
Estimated Time: 5h
Dependencies: Issue 090

## Issue 092: Add Prometheus metrics exporter

Description: Expose core SQL Lens metrics in Prometheus format.

Acceptance Criteria:

- Metrics names match `PLUGIN.md`.
- Labels are low-cardinality.
- Endpoint can be enabled by config.

Labels: `area:observability`, `area:exporter`, `type:feature`
Priority: P2
Difficulty: Medium
Estimated Time: 6h
Dependencies: Issue 025

## Issue 093: Add OpenTelemetry exporter design

Description: Add initial OpenTelemetry exporter configuration and design stubs.

Acceptance Criteria:

- Config fields are documented.
- Metrics export path is sketched.
- No raw SQL is exported without redaction settings.

Labels: `area:observability`, `area:exporter`, `type:design`
Priority: P3
Difficulty: Medium
Estimated Time: 4h
Dependencies: Issue 025

## Issue 094: Add GitHub Actions CI for Rust

Description: Add CI jobs for Rust format, lint, and tests.

Acceptance Criteria:

- `cargo fmt` job exists.
- `cargo clippy` job exists.
- `cargo test` job exists.

Labels: `area:ci`, `type:task`
Priority: P0
Difficulty: Easy
Estimated Time: 3h
Dependencies: Issue 001

## Issue 095: Add GitHub Actions CI for frontend

Description: Add CI jobs for frontend lint, typecheck, and test.

Acceptance Criteria:

- Install step is cached.
- Typecheck job exists.
- Test job exists.

Labels: `area:ci`, `area:frontend`, `type:task`
Priority: P1
Difficulty: Easy
Estimated Time: 3h
Dependencies: Issue 064

## Issue 096: Add markdown documentation lint job

Description: Add CI validation for markdown formatting and links.

Acceptance Criteria:

- Markdown files are checked.
- Broken local links fail CI.
- Rules are documented.

Labels: `area:ci`, `area:docs`, `type:task`
Priority: P2
Difficulty: Easy
Estimated Time: 3h
Dependencies: None

## Issue 097: Add benchmark harness for proxy overhead

Description: Add a benchmark harness that compares direct database latency with proxied latency.

Acceptance Criteria:

- Benchmark command is documented.
- Output includes p50, p95, and p99.
- Direct and proxied results are both captured.

Labels: `area:benchmark`, `type:test`
Priority: P2
Difficulty: Hard
Estimated Time: 8h
Dependencies: Issue 015, Issue 059

## Issue 098: Add release packaging plan

Description: Document and script the first release packaging flow.

Acceptance Criteria:

- Binary release targets are listed.
- Docker image plan is documented.
- Homebrew tap plan is documented.

Labels: `area:release`, `type:task`
Priority: P2
Difficulty: Medium
Estimated Time: 5h
Dependencies: Issue 094

## Issue 099: Add OpenAPI generation

Description: Generate OpenAPI documentation from API schemas or handlers.

Acceptance Criteria:

- `docs/openapi/sql-lens.v1.yaml` is generated.
- REST endpoints from `API.md` are represented.
- CI can detect stale OpenAPI output.

Labels: `area:api`, `area:docs`, `type:feature`
Priority: P2
Difficulty: Medium
Estimated Time: 6h
Dependencies: Issue 028, Issue 029, Issue 030, Issue 031

## Issue 100: Add PostgreSQL protocol research document

Description: Research PostgreSQL protocol support and map it to the shared capture model.

Acceptance Criteria:

- Document covers StartupMessage, Simple Query, Extended Query, Parse, Bind, Execute, Sync, and ErrorResponse.
- Differences from MySQL prepared statements are explained.
- Follow-up implementation issues are proposed.

Labels: `area:protocol-postgresql`, `type:research`
Priority: P2
Difficulty: Medium
Estimated Time: 6h
Dependencies: Issue 019

## Issue 101: Add SQLite integration feasibility document

Description: Research feasible SQLite support approaches.

Acceptance Criteria:

- Document compares driver shim, tracing, log ingestion, and interception approaches.
- Security and portability risks are listed.
- Recommendation for first experiment is provided.

Labels: `area:protocol-sqlite`, `type:research`
Priority: P3
Difficulty: Medium
Estimated Time: 6h
Dependencies: Issue 019

## Issue 102: Add ClickHouse support feasibility document

Description: Research ClickHouse native protocol and HTTP SQL interface support paths.

Acceptance Criteria:

- Document compares native and HTTP approaches.
- Capture model implications are described.
- Recommended first adapter path is proposed.

Labels: `area:protocol-clickhouse`, `type:research`
Priority: P3
Difficulty: Medium
Estimated Time: 6h
Dependencies: Issue 019

## Issue 103: Add XSS regression tests for SQL rendering

Description: Ensure SQL text and database error messages are rendered safely in the UI.

Acceptance Criteria:

- Malicious SQL text does not execute as HTML or JavaScript.
- Database error messages are escaped.
- Tests cover SQL List and SQL Detail.

Labels: `area:frontend`, `area:security`, `type:test`
Priority: P0
Difficulty: Medium
Estimated Time: 5h
Dependencies: Issue 070, Issue 074

## Issue 104: Add CSRF protection for mutating endpoints

Description: Add CSRF protection for replay execute and future settings mutations when cookie auth is enabled.

Acceptance Criteria:

- CSRF token validation exists for mutating endpoints.
- Safe methods are not blocked.
- Tests cover missing and valid tokens.

Labels: `area:api`, `area:security`, `type:feature`
Priority: P1
Difficulty: Medium
Estimated Time: 5h
Dependencies: Issue 026

## Issue 105: Add local web authentication

Description: Add optional local authentication for the web UI and API.

Acceptance Criteria:

- Auth can be enabled by config.
- Session cookie uses safe defaults.
- Loopback-only unauthenticated mode remains possible.

Labels: `area:security`, `area:api`, `type:feature`
Priority: P2
Difficulty: Hard
Estimated Time: 8h
Dependencies: Issue 026, Issue 007

## Issue 106: Add RBAC model skeleton

Description: Add role definitions for viewer, operator, and admin.

Acceptance Criteria:

- Roles are represented in auth context.
- Replay execute requires operator or admin.
- Settings mutation requires admin.

Labels: `area:security`, `type:feature`
Priority: P3
Difficulty: Medium
Estimated Time: 5h
Dependencies: Issue 105

## Issue 107: Add docs website homepage design

Description: Create a documentation page describing the future website homepage layout and content.

Acceptance Criteria:

- Homepage emphasizes product identity in first viewport.
- Includes screenshot area, quick start, architecture, and roadmap sections.
- Does not replace the usable app UI.

Labels: `area:docs`, `area:website`, `type:design`
Priority: P3
Difficulty: Easy
Estimated Time: 3h
Dependencies: None

## Issue 108: Add logo exploration brief

Description: Document logo direction for SQL Lens.

Acceptance Criteria:

- Brief covers lens, query, database, and observability concepts.
- Includes light and dark usage notes.
- Avoids vendor-specific database imagery.

Labels: `area:design`, `type:design`
Priority: P3
Difficulty: Easy
Estimated Time: 2h
Dependencies: None

## Issue 109: Add release notes template

Description: Add a template for GitHub release notes.

Acceptance Criteria:

- Template includes highlights, breaking changes, compatibility, security, and upgrade notes.
- SemVer guidance is included.
- Example release note is provided.

Labels: `area:release`, `area:docs`, `type:task`
Priority: P2
Difficulty: Easy
Estimated Time: 3h
Dependencies: Issue 098

## Issue 110: Add contributor good-first-issue guide

Description: Create a guide for selecting first contribution tasks.

Acceptance Criteria:

- Guide maps labels to recommended contributor skill levels.
- Includes setup and validation expectations.
- Links to issue backlog categories.

Labels: `area:docs`, `type:task`, `good-first-issue`
Priority: P2
Difficulty: Easy
Estimated Time: 3h
Dependencies: None
