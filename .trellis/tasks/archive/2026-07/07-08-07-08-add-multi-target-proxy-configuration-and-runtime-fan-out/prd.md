# Add Multi-Target Proxy Configuration and Runtime Fan-Out

## Goal

Support running one SQL Lens process with multiple explicitly configured
MySQL-compatible proxy targets at the same time, such as one local listener for
MySQL and another local listener for StarRocks.

The product value is being able to debug applications that talk to more than one
database surface without running a separate SQL Lens process per backend.

## Confirmed Facts

- Current config is single-target: one `[proxy]` and one `[backend]`.
- Current app runtime binds one proxy listener and dials one backend.
- Current `ApiState` can hold events from many connections, but target identity
  is not modeled as a first-class runtime concept.
- Existing SQL event contracts already include protocol, database type,
  backend address, and protocol metadata.
- The prime directive still applies: SQL Lens observes and captures; it must not
  become a general database middleware.

## Requirements

- R1. Add a multi-target configuration shape that can represent multiple
  `(name, listen, protocol, database_type, backend_address)` entries.
- R2. Preserve backwards compatibility for the existing single `[proxy]` plus
  `[backend]` configuration.
- R3. Validate each target independently: name, listener address, backend
  address, and supported protocol.
- R4. Reject duplicate target names and duplicate listen addresses.
- R5. Start one proxy listener per effective target in runtime composition.
- R6. Share one API state across all target listeners so SQL events can be
  viewed together.
- R7. Captured `ConnectionInfo` and `SqlEvent` values must carry the target's
  correct `database_type` and enough target identity for API/frontend display.
- R8. Keep routing explicit by listener port. Do not implement dynamic routing
  by SQL text, username, SNI, database name, or packet contents.
- R9. Update backend and frontend architecture specs to document the multi-target
  contract and UI adaptation boundary.
- R10. Add a frontend follow-up issue to `ISSUES.md`; frontend implementation is
  out of scope for this backend task.

## Acceptance Criteria

- [x] Config supports multiple named proxy targets.
- [x] Existing single-target config continues to load and validate.
- [x] Duplicate target names are rejected.
- [x] Duplicate target listen addresses are rejected.
- [x] Unsupported target protocol is rejected.
- [x] Runtime can bind and run multiple listeners sharing one API state.
- [x] Events captured through different targets have correct database type.
- [x] Target identity is exposed through a stable backend-owned event/API design.
- [x] Backend architecture spec documents explicit multi-target listener fan-out
      and forbidden middleware behaviors.
- [x] Frontend architecture spec documents target-aware UI/API type expectations
      without requiring frontend implementation in this task.
- [x] `ISSUES.md` contains a frontend follow-up issue for target selection /
      multi-target display adaptation.
- [x] `rtk cargo fmt --check` passes.
- [x] `rtk cargo test --workspace` passes.
- [x] `rtk cargo clippy --workspace --all-targets -- -D warnings` passes.

## Out Of Scope

- Dynamic routing through one listener.
- SQL rewrite, sharding, load balancing, failover, read/write splitting, or
  policy enforcement.
- Frontend implementation.
- Non-MySQL protocol adapters.
- Persistent storage schema changes unless needed for target identity in the
  in-memory/API contract.

## Implementation Status

Implemented explicit multi-target proxy fan-out, not middleware routing. The
backend exposes protocol-neutral `target_name` on captured events and
connections, supports target-aware REST/WebSocket event contracts, and keeps
frontend implementation out of scope.
