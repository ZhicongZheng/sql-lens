# SQL Lens Milestones

## Milestone 1: Repository And Workspace Foundation

Goal: establish project shape.

Tasks:

- Create Rust workspace.
- Create crate skeletons.
- Create web app skeleton.
- Add formatting and linting configuration.
- Add CI design.
- Add basic CLI entry point.

Deliverable: contributors can build an empty SQL Lens binary and web shell.

## Milestone 2: Configuration And Runtime Startup

Goal: load configuration and start services.

Tasks:

- Implement TOML config model.
- Add environment overrides.
- Validate proxy and backend settings.
- Initialize logging.
- Start and stop runtime services.

Deliverable: SQL Lens starts with a config file and exposes health status.

## Milestone 3: TCP Proxy Foundation

Goal: forward bytes between client and backend.

Tasks:

- Bind proxy listener.
- Accept client connections.
- Dial backend.
- Forward client-to-backend bytes.
- Forward backend-to-client bytes.
- Track bytes and connection lifecycle.

Deliverable: a client can connect through SQL Lens to a database.

## Milestone 4: Capture Event Model

Goal: define protocol-neutral event data.

Tasks:

- Define `SqlEvent`.
- Define `ConnectionInfo`.
- Define `SqlParameter`.
- Define `QueryTiming`.
- Define `ProtocolMetadata`.
- Add serialization tests.

Deliverable: shared event model is stable enough for storage and API.

## Milestone 5: Ring Buffer Storage

Goal: store recent events in memory.

Tasks:

- Implement append.
- Implement detail lookup.
- Implement timeline query.
- Implement filters.
- Implement eviction.
- Add statistics helpers.

Deliverable: API can query captured events from memory.

## Milestone 6: MySQL Packet Framing

Goal: parse MySQL-compatible packet envelopes.

Tasks:

- Decode packet header.
- Track payload length.
- Track sequence ID.
- Support packet fixture tests.
- Handle malformed packets safely.

Deliverable: packet observer can read MySQL frame boundaries.

## Milestone 7: MySQL Handshake And Login Observation

Goal: observe connection setup without storing secrets.

Tasks:

- Observe server handshake.
- Observe client handshake response.
- Track auth state.
- Detect auth success or failure.
- Redact authentication payloads.

Deliverable: connection records show authentication state safely.

## Milestone 8: COM_QUERY Capture

Goal: capture text SQL queries.

Tasks:

- Parse `COM_QUERY`.
- Capture SQL text.
- Track command timing.
- Parse OK and error summaries.
- Emit SQL events.

Deliverable: text queries appear in REST and WebSocket output.

## Milestone 9: Prepared Statement Prepare

Goal: track statement templates.

Tasks:

- Parse `COM_STMT_PREPARE`.
- Capture template SQL.
- Parse backend prepare response.
- Store statement ID per connection.
- Handle prepare errors.

Deliverable: prepared statement templates are available for later execution.

## Milestone 10: Prepared Statement Execute

Goal: decode parameters and expanded SQL.

Tasks:

- Parse `COM_STMT_EXECUTE`.
- Decode common parameter types.
- Render expanded SQL.
- Handle NULL bitmap.
- Handle unsupported values.
- Add fixture tests.

Deliverable: prepared statement executions show parameters and expanded SQL.

## Milestone 11: REST API

Goal: expose queryable state.

Tasks:

- Implement health endpoint.
- Implement SQL event list.
- Implement SQL event detail.
- Implement connections endpoint.
- Implement statistics endpoint.
- Standardize error responses.

Deliverable: UI can read all core data via API.

## Milestone 12: WebSocket Live Events

Goal: stream live SQL activity.

Tasks:

- Add `/ws/sql`.
- Add subscription filters.
- Broadcast capture events.
- Add backpressure handling.
- Add dropped-event counters.

Deliverable: UI can update live without polling.

## Milestone 13: Web UI Shell

Goal: create usable app frame.

Tasks:

- Add React app.
- Add routing.
- Add layout.
- Add API client.
- Add WebSocket client.
- Add theme support.

Deliverable: web app shell loads and connects to API.

## Milestone 14: Dashboard And SQL List

Goal: provide primary debugging surface.

Tasks:

- Add dashboard metrics.
- Add SQL timeline.
- Add filters.
- Add live pause.
- Add status indicators.

Deliverable: users can watch and filter SQL events.

## Milestone 15: SQL Detail And Connections

Goal: inspect one event or connection deeply.

Tasks:

- Add SQL detail page.
- Add Monaco SQL display.
- Add parameter table.
- Add connection list.
- Add connection detail.

Deliverable: users can understand individual SQL executions.

## Milestone 16: Slow SQL, Error SQL, Search, Replay Preview

Goal: complete v1 debugging workflows.

Tasks:

- Add slow classification.
- Add error classification.
- Add text and structured search.
- Add replay preview API.
- Add replay UI safeguards.

Deliverable: SQL Lens is useful as a local SQL debugger.

## Milestone 17: v1 Release Readiness

Goal: make the project releasable.

Tasks:

- Complete docs.
- Complete CI.
- Run compatibility tests.
- Run benchmarks.
- Package binaries.
- Publish release notes.

Deliverable: v1.0 release candidate.

