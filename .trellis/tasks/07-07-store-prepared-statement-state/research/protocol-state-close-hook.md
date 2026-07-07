# Protocol state close hook research

## Evidence

`crates/sql-lens-protocol/src/adapter.rs` defines:

```rust
pub trait ProtocolAdapter: fmt::Debug + Send + Sync {
    fn protocol_name(&self) -> ProtocolName;
    fn create_connection_state(
        &self,
        context: &ProtocolConnectionContext,
    ) -> Box<dyn ProtocolConnectionState>;
    fn observe_client_bytes(...);
    fn observe_backend_bytes(...);
}

pub trait ProtocolConnectionState: Any + fmt::Debug + Send {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
```

There is no close callback in the shared protocol adapter contract.

## Scope Decision Needed

Issue 049 says statement state is removed on connection close. The narrow implementation can satisfy this by storing the map inside `MysqlConnectionState`, because the entire state is per connection and drops on connection close.

Adding an explicit close hook would require shared protocol trait design and changes outside the MySQL crate. That should be a separate task if needed.
