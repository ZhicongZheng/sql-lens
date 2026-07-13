# Parent Design

The parent task is a sequence of independent runtime composition improvements. Each child owns one boundary and must preserve the existing crate ownership rules. The first child establishes connection lifecycle tracking, the second establishes one redaction policy source, and the third depends on both configuration and storage behavior. Replay, protocol registry integration, and plugin dispatch are later because they depend on stable runtime/session boundaries.

## Dependency Order

```text
proxy governance -> redaction wiring -> retention completion
                                      -> replay execution
                                      -> protocol registry runtime
                                      -> plugin runtime
```

The last three can be developed independently after the first two, but will be executed sequentially in this session to keep runtime changes easy to verify.

## Rollback

Each child is independently revertible by its task-scoped changes. Runtime flags should fail closed: unsupported protocol/storage/plugin configuration returns a clear startup error rather than silently claiming support.
