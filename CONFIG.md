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
slow_threshold_ms = 500
max_connections = 512
connect_timeout_ms = 5000
idle_timeout_ms = 300000
shutdown_timeout_ms = 10000

[capture]
capacity = 1024
overload_policy = "drop_newest"

[backend]
address = "127.0.0.1:3306"
database_type = "mysql"

# Optional multi-target form. When present, SQL Lens starts one proxy listener
# per target and ignores the legacy [proxy] + [backend] pair for runtime target
# expansion.
#
# [[targets]]
# name = "mysql-local"
# listen = "127.0.0.1:3307"
# protocol = "mysql"
# database_type = "mysql"
# backend_address = "127.0.0.1:3306"
#
# [[targets]]
# name = "starrocks-local"
# listen = "127.0.0.1:9037"
# protocol = "mysql"
# database_type = "starrocks"
# backend_address = "127.0.0.1:9030"

[tls]
mode = "passthrough"
client_cert_path = ""
client_key_path = ""
ca_cert_path = ""

[web]
listen = "127.0.0.1:5173"
base_url = "http://127.0.0.1:5173"
cors_origins = ["http://127.0.0.1:5173"]
static_dir = "crates/sql-lens-app/web/dist"

[storage]
type = "ring_buffer"
capacity = 100000
path = ""

[retention]
max_age = "24h"
max_events = 100000
enforcement_enabled = true
enforcement_interval = "1h"

[logging]
level = "info"
format = "json"
redact_secrets = true

[redaction]
enabled = true
mask = "***"
parameter_names = ["password", "token", "secret"]
sql_patterns = []

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
- `slow_threshold_ms`: successful SQL at or above this duration is classified
  as `slow`.
- `max_connections`: connection limit.
- `connect_timeout_ms`: backend dial timeout.
- `idle_timeout_ms`: idle connection timeout.
- `shutdown_timeout_ms`: maximum time to drain active proxy sessions during shutdown.

### `backend`

Controls the upstream database.

### `capture`

Controls the bounded handoff from protocol observation to runtime fan-out.

- `capacity`: maximum number of normalized events queued in memory; must be
  greater than zero.
- `overload_policy`: `drop_newest` drops an incoming event when the queue is
  full; `reject_new` reports the rejected event to runtime diagnostics. Neither
  policy blocks packet forwarding.

Fields:

- `address`: host and port.
- `database_type`: `mysql`, `starrocks`, `tidb`, `doris`, or future values.
- `tls_server_name`: optional backend TLS name.

### `targets`

Optional multi-target proxy configuration. Use this when one SQL Lens process
should observe multiple explicit database surfaces, such as MySQL and
StarRocks, at the same time.

Each `[[targets]]` entry owns exactly one listener and one backend:

- `name`: stable protocol-neutral target name exposed in API events.
- `listen`: bind address for this target's database client connections.
- `protocol`: currently supported value is `mysql`.
- `database_type`: `mysql`, `starrocks`, `tidb`, `doris`, or future values.
- `backend_address`: upstream host and port for this target.

The legacy `[proxy]` plus `[backend]` shape remains valid and expands to one
effective target named `default` when `[[targets]]` is absent.

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
- `static_dir`: optional built frontend directory. When set, `sql-lens` serves
  the UI and SPA routes from this directory on the same listener as its API and
  WebSocket endpoints. The directory must contain `index.html`. When omitted,
  the runtime auto-discovers a built UI at `crates/sql-lens-app/web/dist` or
  `web/dist` relative to the process working directory (first match with
  `index.html`). If none is found, the process stays API-only.
  Build the UI with `./scripts/build-web.sh` or
  `cd crates/sql-lens-app/web && npm install && npm run build`.
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
- `enforcement_enabled`: starts periodic runtime cleanup when true.
- `enforcement_interval`: positive `ms`, `s`, `m`, or `h` duration between
  cleanup runs; the default is `1h`.
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

Supported overrides:

- `SQL_LENS_PROXY_LISTEN`: overrides legacy `[proxy].listen`.
- `SQL_LENS_BACKEND_ADDRESS`: overrides legacy `[backend].address`.
- `SQL_LENS_LOGGING_LEVEL`: overrides `[logging].level`; valid values are
  `trace`, `debug`, `info`, `warn`, and `error`.

When explicit `[[targets]]` are configured, legacy proxy/backend overrides do
not rewrite target entries. Use the TOML `[[targets]]` block for multi-target
local setups.

## Validation Rules

- `proxy.listen` is required.
- `backend.address` is required for proxy mode.
- `proxy.protocol` must match an installed adapter.
- When `[[targets]]` is present, every target requires `name`, `listen`, and
  `backend_address`.
- Target names and listen addresses must be unique.
- Target protocol must be `mysql` in the current build.
- `storage.capacity` must be positive for ring buffer.
- TLS certificate paths are required for TLS termination.
