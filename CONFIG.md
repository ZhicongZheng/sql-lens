# SQL Lens Configuration

## Overview

SQL Lens uses a single configuration file plus environment variable overrides.

Recommended format: TOML.

Default path:

```text
sql-lens.toml
```

Environment variable prefix:

```text
SQL_LENS_
```

## Example

```toml
[proxy]
listen = "127.0.0.1:3307"
protocol = "mysql"
capture_mode = "observe"
max_connections = 512
connect_timeout_ms = 5000
idle_timeout_ms = 300000
shutdown_timeout_ms = 10000

[backend]
address = "127.0.0.1:3306"
database_type = "mysql"

[tls]
mode = "passthrough"
client_cert_path = ""
client_key_path = ""
ca_cert_path = ""

[web]
listen = "127.0.0.1:5173"
base_url = "http://127.0.0.1:5173"
cors_origins = ["http://127.0.0.1:5173"]

[storage]
type = "ring_buffer"
capacity = 100000
path = ""

[retention]
max_age = "24h"
max_events = 100000

[logging]
level = "info"
format = "json"
redact_secrets = true

[redaction]
enabled = true
mask = "***"
parameter_names = ["password", "token", "secret"]
sql_patterns = []

[auth]
enabled = false
mode = "local"
session_ttl = "12h"

[replay]
enabled = true
require_confirmation_for_mutations = true

[plugins]
enabled = false
directory = "plugins"
```

## Sections

### `proxy`

Controls the database proxy listener.

Fields:

- `listen`: bind address for database client connections.
- `protocol`: initial protocol adapter, such as `mysql`.
- `capture_mode`: `observe` for normal capture.
- `max_connections`: connection limit.
- `connect_timeout_ms`: backend dial timeout.
- `idle_timeout_ms`: idle connection timeout.
- `shutdown_timeout_ms`: maximum time to drain active proxy sessions during shutdown.

### `backend`

Controls the upstream database.

Fields:

- `address`: host and port.
- `database_type`: `mysql`, `starrocks`, `tidb`, `doris`, or future values.
- `tls_server_name`: optional backend TLS name.

### `tls`

TLS modes:

- `disabled`: plain TCP.
- `passthrough`: forward TLS without decrypting SQL. Capture is limited.
- `terminate`: SQL Lens terminates client TLS.
- `upstream`: SQL Lens connects to backend over TLS.

TLS termination must be explicit because it changes the trust boundary.

### `web`

Controls REST API and UI.

Fields:

- `listen`.
- `base_url`.
- `cors_origins`.
- `static_dir`.
- `request_timeout_ms`.

### `storage`

Storage backend:

- `ring_buffer`: in-memory default.
- `sqlite`: optional local persistence.
- `duckdb`: future analytics storage.

### `retention`

Retention policy:

- `max_age`.
- `max_events`.
- `max_bytes`.
- `drop_policy`: `oldest` or `reject_new`.

### `logging`

Fields:

- `level`: `trace`, `debug`, `info`, `warn`, `error`.
- `format`: `json` or `pretty`.
- `redact_secrets`: must default to true.

### `redaction`

Controls SQL and parameter redaction.

Rules:

- Redaction happens before storage and UI broadcast.
- Raw sensitive values should not be recoverable from storage.
- Configuration should support parameter names, SQL regex patterns, and future classifier plugins.

### `auth`

Auth modes:

- `disabled`: local development only.
- `local`: username/password for shared local network usage.
- `oidc`: future enterprise or team mode.

### `plugins`

Controls plugin loading.

Fields:

- `enabled`.
- `directory`.
- `allow_network`.
- `timeout_ms`.

## Environment Overrides

Examples:

```bash
SQL_LENS_PROXY_LISTEN=127.0.0.1:3308
SQL_LENS_BACKEND_ADDRESS=127.0.0.1:3306
SQL_LENS_LOGGING_LEVEL=debug
```

## Validation Rules

- `proxy.listen` is required.
- `backend.address` is required for proxy mode.
- `proxy.protocol` must match an installed adapter.
- `storage.capacity` must be positive for ring buffer.
- TLS certificate paths are required for TLS termination.
- `auth.enabled=true` requires a configured auth mode.
