# Implement Plugin Loading And Hook Dispatch

## Goal

Turn the existing protocol-neutral plugin hook traits into a safe runtime dispatcher with explicit loading and failure isolation.

## Requirements

- Load plugins only when `plugins.enabled` is true.
- Define a stable loading boundary for the configured plugin directory.
- Dispatch redacted connection, query, prepare, execute, and error payloads.
- Keep plugin failures from stopping packet forwarding or capture delivery.
- Enforce configured plugin timeout or explicitly reject unsupported timeout behavior at startup.
- Keep plugin execution off the forwarding hot path where practical.

## Acceptance Criteria

- Disabled plugins cause no plugin loading or dispatch.
- A valid test plugin receives redacted hook payloads.
- A failing or timing-out plugin is logged/isolated and proxy forwarding continues.
- Missing or malformed plugin artifacts return clear startup errors when plugins are enabled.
- Hook dispatch order and shutdown behavior are tested.

## Out Of Scope

- Remote plugin installation.
- Arbitrary native-code loading without a documented safety boundary.
- Exporter implementations such as Prometheus or OpenTelemetry.
