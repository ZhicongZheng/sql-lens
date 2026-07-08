# Apply Redaction Before Storage - Design

## Architecture

Redaction should live in `sql-lens-core` as a protocol-neutral event
transformation:

```rust
pub struct RedactionPolicy {
    pub enabled: bool,
    pub mask: String,
    pub parameter_names: Vec<String>,
    pub sql_patterns: Vec<String>,
}

pub fn redact_sql_event(event: SqlEvent, policy: &RedactionPolicy) -> SqlEvent;
```

`sql-lens-core` is the right owner because:

- `SqlEvent` and `SqlParameter` are protocol-neutral core contracts.
- `sql-lens-storage` depends on core and can call redaction before retention.
- `sql-lens-api` depends on core and can call redaction before broadcast.
- `sql-lens-config` must not become a dependency of core.

The default policy should be available from core:

- `enabled = true`
- `mask = "***"`
- `parameter_names = ["password", "passwd", "token", "secret", "api_key",
  "access_key", "refresh_token"]`
- `sql_patterns = []`

Config integration is a later composition concern. This issue may update
`RedactionConfig::default()` to match the same documented sensitive names, but
it should not introduce app-level hot reload or runtime config plumbing.

## Data Flow

Current flow has no central capture fan-out layer:

```text
SqlEvent producer
  -> RingBufferStore::append(event)
  -> RingBufferStore retains event
  -> REST endpoints serialize retained event

SqlEvent producer
  -> SqlEventBroadcaster::publish(event)
  -> WebSocket subscriber receives event
  -> sql_event.created serializes event summary
```

Required first-version flow:

```text
SqlEvent producer
  -> RingBufferStore::append(event)
  -> redact_sql_event(event, store_policy)
  -> RingBufferStore retains redacted event
  -> REST endpoints serialize redacted retained event

SqlEvent producer
  -> SqlEventBroadcaster::publish(event)
  -> redact_sql_event(event, broadcaster_policy)
  -> WebSocket subscriber receives redacted event
  -> sql_event.created serializes redacted event summary
```

This duplicates the redaction call at two sink boundaries by design. It is a
defense-in-depth step until a later capture consumer/fan-out task creates one
central redaction point before cloning events to sinks.

## Redaction Semantics

### Policy Disabled

If `policy.enabled == false`, return the event unchanged.

### Parameter Names

Parameter names match by case-insensitive exact comparison using the configured
`parameter_names` list. Empty configured names are ignored.

When a parameter matches, or when `parameter.redacted` is already true:

- Set `parameter.redacted = true`.
- Replace `parameter.value` with `SqlParameterValue::String(policy.mask)`.
- Record the original value as a SQL text redaction candidate before replacing
  it, unless the original value is empty or `NULL`.

This keeps the existing API schema compatible. Consumers already have the
`redacted` flag to distinguish masked values.

### SQL Text Patterns

Apply `policy.sql_patterns` as literal substring replacements to:

- `original_sql`
- `normalized_sql`
- `expanded_sql`

Rules:

- Empty patterns are ignored.
- Replacement is deterministic `str::replace`.
- No regex, no glob syntax, and no SQL parsing in this issue.

### Expanded SQL Parameter Values

For every parameter that is redacted because of name or pre-existing redaction
state, remove its original display value from SQL text fields as well.

The core redactor should build simple text replacement pairs from the original
`SqlParameterValue`:

- String-like values: raw value -> mask, SQL single-quoted display literal ->
  SQL single-quoted mask.
- Numeric values: rendered number -> mask.
- Boolean values: `TRUE` or `FALSE` -> mask.
- `NULL`: no SQL text replacement candidate.
- Binary summaries and unsupported values: treat as string-like display text.

This is conservative and may redact a repeated literal that happens to match
the sensitive value. That is acceptable for this security boundary. It is also
why the task avoids claiming exact replay fidelity for expanded SQL.

## Storage Boundary

`RingBufferStore` should own a `RedactionPolicy`:

```rust
pub struct RingBufferStore {
    redaction_policy: RedactionPolicy,
    // existing fields
}
```

Recommended constructors:

```rust
pub fn new(capacity: NonZeroUsize) -> Self;
pub fn with_redaction_policy(
    capacity: NonZeroUsize,
    redaction_policy: RedactionPolicy,
) -> Self;
```

`new` uses `RedactionPolicy::default()`. `append` redacts before assigning the
sequence entry:

```rust
let event = redact_sql_event(event, &self.redaction_policy);
```

All existing query APIs should continue to operate on the retained event. This
means text search sees redacted text, which is the correct privacy-preserving
behavior for this first version.

## WebSocket Boundary

`SqlEventBroadcaster` should own a `RedactionPolicy`:

```rust
pub struct SqlEventBroadcaster {
    redaction_policy: RedactionPolicy,
    // existing fields
}
```

Recommended constructors:

```rust
pub fn new(capacity: NonZeroUsize) -> Self;
pub fn with_redaction_policy(
    capacity: NonZeroUsize,
    redaction_policy: RedactionPolicy,
) -> Self;
```

`new` uses `RedactionPolicy::default()`. `publish` redacts before sending on the
broadcast channel.

Current WebSocket filters use protocol, status, database, and duration, so
redaction should not change filter semantics.

## API Boundary

REST endpoints read from `RingBufferStore`, so they inherit storage redaction.
Do not add separate response-level redaction in this issue; duplicating logic at
serialization time would make storage and API behavior diverge.

Future API-only redaction modes can be added when the product supports roles
or privileged unmasking.

## Compatibility

- `SqlEvent` and API response field names stay unchanged.
- `SqlParameterValue` variants stay unchanged.
- `RingBufferStore::new` and `SqlEventBroadcaster::new` remain available.
- Tests that append non-sensitive events should keep passing because default
  redaction does not change values unless a sensitive name, pre-redacted flag,
  or SQL pattern is present.

## Trade-Offs

- Literal replacement is simpler and safer for this milestone than regex, but
  it cannot express advanced redaction patterns.
- Replacing sensitive parameter values in SQL text can mask repeated literals
  that are not the parameter instance. This is acceptable because the output is
  a debugging display, not replay SQL.
- Keeping redaction in core avoids duplicate storage/API implementations, but
  runtime config conversion must wait for a later composition task.

## Rollback

If the implementation causes broad regressions, revert the storage and
broadcaster policy fields first. The core redaction module can remain unused
until the boundary wiring is corrected.
