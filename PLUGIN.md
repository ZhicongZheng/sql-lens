# SQL Lens Plugin System

## Overview

The plugin system lets users extend SQL Lens without modifying proxy internals.

Plugins should observe and export events. They must not sit in the hot forwarding path unless explicitly designed and bounded.

## Goals

- Export captured SQL events.
- Add custom redaction.
- Add custom classification.
- Send webhooks.
- Publish metrics.
- Integrate with OpenTelemetry.
- Preserve core proxy safety.

## Non-Goals

- Query rewriting in the open source MVP.
- Arbitrary packet mutation.
- Unbounded synchronous hooks on the forwarding path.
- Plugins with implicit access to secrets.

## Hook Points

### `OnConnect`

Called when a client/backend connection is established.

Input:

- Connection info.
- Protocol.
- Client address.
- Backend address.

Use cases:

- Connection audit.
- Tagging.
- Metrics.

### `OnQuery`

Called when a text query is captured.

Input:

- SQL event draft.
- Connection info.
- Protocol metadata.

Use cases:

- Classification.
- Export.
- Redaction.

### `OnPrepare`

Called when a prepared statement is created.

Input:

- Statement key.
- SQL template.
- Parameter count.
- Connection info.

Use cases:

- Statement cataloging.
- Fingerprinting.

### `OnExecute`

Called when a prepared statement is executed.

Input:

- SQL event.
- Parameter list.
- Expanded SQL.

Use cases:

- Export.
- Redaction.
- Slow query tagging.

### `OnError`

Called when an error SQL event is captured.

Input:

- Error summary.
- SQL event.
- Connection info.

Use cases:

- Alerting.
- Webhook.
- Error aggregation.

## Exporter Interface

Exporter types:

- File exporter.
- Webhook exporter.
- Prometheus exporter.
- OpenTelemetry exporter.
- Future custom exporters.

Exporter rules:

- Exporters receive redacted events by default.
- Exporters should be asynchronous.
- Exporter failures are recorded but do not stop proxy forwarding.
- Retry policies must be bounded.

## Webhook

Webhook payload:

```json
{
  "type": "sql_event.created",
  "version": 1,
  "event": {
    "id": "evt_01J00000000000000000000000",
    "protocol": "mysql",
    "status": "slow",
    "duration_ms": 180.0,
    "expanded_sql": "SELECT * FROM orders WHERE id = 42"
  }
}
```

Webhook requirements:

- Timeout.
- Retry limit.
- Redacted payloads.
- Signature header.

## Prometheus

Prometheus exporter should expose:

- `sql_lens_queries_total`.
- `sql_lens_query_duration_seconds`.
- `sql_lens_errors_total`.
- `sql_lens_slow_queries_total`.
- `sql_lens_connections_active`.
- `sql_lens_capture_events_dropped_total`.

Labels should be low-cardinality:

- `protocol`.
- `database_type`.
- `status`.

Avoid labels with raw SQL, user IDs, or client IPs by default.

## OpenTelemetry

OpenTelemetry support should emit:

- Metrics.
- Optional traces.
- Optional logs.

Trace integration may map SQL events to spans, but raw SQL should be controlled by redaction settings.

## Plugin Isolation

Initial implementation can use in-process plugins only if tightly controlled.

Future safer options:

- WASM plugins.
- External process plugins over a local protocol.
- Export-only integrations.

## Configuration

```toml
[plugins]
enabled = false
directory = "plugins"
timeout_ms = 100
allow_network = false

[[plugins.exporters]]
type = "webhook"
url = "https://example.com/sql-lens"
events = ["error", "slow"]
```

## Stability

Plugin APIs should be versioned separately from internal Rust types.

Rules:

- Public plugin payloads use stable schemas.
- Internal struct changes must not break plugin payloads without versioning.
- Plugin hooks should receive protocol-neutral events plus optional metadata.

