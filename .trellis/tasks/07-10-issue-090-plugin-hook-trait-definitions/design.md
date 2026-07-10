# Issue 090 Design: Plugin Hook Contracts

## Scope

This task adds the public in-process hook contracts in
`crates/sql-lens-plugin`. It does not dispatch hooks from the proxy or capture
pipeline and does not load plugin implementations.

## Boundary

`sql-lens-plugin` depends on `sql-lens-core` for stable, protocol-neutral
payload models:

- `OnConnect` receives `&ConnectionInfo`.
- `OnQuery` receives `&SqlEvent` and `&ConnectionInfo`.
- `OnPrepare` receives `&PreparedStatementInfo` and `&ConnectionInfo`.
- `OnExecute` receives `&SqlEvent` and `&ConnectionInfo`.
- `OnError` receives `&SqlEvent`, `&ConnectionInfo`, and `&ErrorSummary`.

`ConnectionInfo` already carries protocol, database type, client address, and
backend address. `SqlEvent` already carries protocol metadata, parameters, and
expanded SQL, so no duplicate or MySQL-specific fields are added to plugin
payloads.

## Public Contract

The crate root exposes five independently implementable synchronous traits:

```rust
pub trait OnConnect {
    fn on_connect(&mut self, connection: &ConnectionInfo) -> PluginResult;
}
```

The other traits follow the same object-safe shape with their hook-specific
arguments. `&mut self` permits stateful implementations while keeping methods
free of generic parameters, `Self: Sized` requirements, and async return types.

All hooks return:

```rust
pub type PluginResult = Result<(), PluginError>;
```

`PluginError` is a small owned error containing a diagnostic message and
implements `Debug`, `Display`, `Error`, `Clone`, `PartialEq`, and `Eq`.
Returning an error reports a plugin failure to the future dispatcher; it does
not prescribe retry or shutdown behavior. Runtime isolation remains the
dispatcher’s responsibility and is outside this task.

## Safety And Compatibility

- Hook arguments are borrowed, so contract definitions do not clone events or
  parameters and do not retain caller-owned data by accident.
- Hooks cannot mutate captured events or forwarded traffic through these
  signatures.
- The dispatcher must pass redacted event values before invoking hooks. This
  contract does not attempt to encode redaction state in Rust types.
- No async runtime, HTTP client, serialization layer, dynamic loading, or
  exporter dependency is added.
- Future API versioning is a separate concern; this task keeps the initial
  contract intentionally small and documents that adding required trait methods
  is a compatibility change.

## Documentation

Add a concise Rust contract section to `PLUGIN.md` mapping each documented hook
to its trait and payload models. Clarify that hook invocation is synchronous at
the contract boundary, errors are returned to the dispatcher, and redaction is
required before event delivery.

## Test Strategy

Unit tests in `sql-lens-plugin/src/lib.rs` will:

- construct representative core payloads;
- implement each hook with a small stateful test plugin;
- invoke each hook through `Box<dyn Trait>` values;
- assert successful and failing `PluginResult` behavior; and
- verify a plugin error formats as a standard error.

The trait-object calls provide the object-safety check without introducing a
second registry abstraction before it has a concrete consumer.
