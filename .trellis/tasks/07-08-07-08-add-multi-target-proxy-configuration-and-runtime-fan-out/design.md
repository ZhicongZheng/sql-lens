# Design — Multi-Target Proxy Configuration and Runtime Fan-Out

## Current Limitation

The current backend model is single-target:

- `SqlLensConfig` has one `[proxy]` and one `[backend]`.
- `sql-lens-app` binds one proxy listener and dials one backend.
- Current runtime connection info defaults `database_type` to `mysql` in the
  minimal runtime path.

This works for one backend at a time, but not for the common debugging case of
watching an application that talks to both MySQL and StarRocks.

## Proposed Model

Introduce explicit configured targets. Each target owns one listener and one
backend:

```toml
[[targets]]
name = "mysql-local"
listen = "127.0.0.1:3307"
protocol = "mysql"
database_type = "mysql"
backend_address = "127.0.0.1:3306"

[[targets]]
name = "starrocks-local"
listen = "127.0.0.1:9037"
protocol = "mysql"
database_type = "starrocks"
backend_address = "127.0.0.1:9030"
```

The existing single-target `[proxy]` + `[backend]` shape remains valid and can be
converted into one effective target.

## Runtime Shape

- Build an effective target list from config.
- Start one `TcpProxyListener` per target.
- Reuse one `ApiState` for all target listeners.
- Pass target metadata into connection creation.
- On shutdown, signal all target listener tasks.

## Event Identity

At minimum, captured events must expose:

- Correct `database_type`.
- Existing `backend_addr`.
- Target identity through a stable backend-owned contract, such as
  protocol-neutral metadata field `target_name` or a first-class target field if
  the implementation task explicitly updates the public event/API model.

Do not add MySQL-specific top-level fields. Target identity is protocol-neutral.

## Frontend Boundary

The frontend should not implement this task, but frontend architecture must be
ready for it:

- API types should preserve target identity when the backend exposes it.
- SQL event list/detail views should be able to display and filter by target.
- Target filtering must not replace existing protocol or database type filters.

## Out Of Scope

- Dynamic SQL routing.
- One listener that multiplexes to several backends.
- Load balancing, failover, read/write splitting, sharding, or SQL rewrite.
- Frontend implementation.
- Non-MySQL protocol adapters.
