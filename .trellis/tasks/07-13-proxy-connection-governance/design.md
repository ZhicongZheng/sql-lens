# Proxy Governance Design

## Boundary

Keep listener acceptance in `sql-lens-app`, put reusable drain/timeout primitives in `sql-lens-proxy`, and keep protocol observation inside the MySQL adapter. The app owns a shared active-session registry containing abortable task handles and the configured limit.

## Session Flow

```text
accept -> reserve slot -> dial backend -> register session task
                                     -> observe/forward
                                     -> finalize lifecycle -> release slot
shutdown -> stop listeners -> signal sessions -> drain until deadline -> abort remaining
```

Use an atomic counter or semaphore for admission. Use task handles for shutdown; do not rely on detached `tokio::spawn` tasks. The limit must be released on every dial failure and task completion path.

## Trade-offs

- A Tokio semaphore is the simplest admission control and naturally handles concurrent accepts.
- A session task wrapper can apply `tokio::time::timeout` around forwarding without changing protocol code.
- Reuse `ActiveSessionDrain` where its ownership model fits; otherwise extend it minimally rather than creating a second shutdown abstraction.

## Compatibility

The public `MinimalMysqlRuntime` API remains unchanged. Existing default configuration must preserve current behavior except that shutdown now drains tracked sessions instead of abandoning them.
