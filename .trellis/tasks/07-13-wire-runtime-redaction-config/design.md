# Runtime Redaction Design

Construct one `RedactionPolicy` in app composition and pass it into storage constructors and broadcast boundaries. Storage remains responsible for applying the policy before retaining data; API handlers should not reimplement masking. The policy must be cloned into long-lived stores because runtime configuration is startup-scoped today.

## Data Flow

```text
SqlLensConfig.redaction -> RedactionPolicy
      -> RingBufferStore
      -> SqliteEventStore persistence
      -> live event broadcaster
```

The existing default policy remains the default when callers construct storage directly in tests. Runtime composition is the only required integration boundary.
