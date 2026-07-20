//! Protocol adapter contracts for SQL Lens.

mod adapter;
mod registry;

pub use adapter::{
    CaptureEventEmitter, ProtocolAdapter, ProtocolAdapterError, ProtocolConnectionContext,
    ProtocolConnectionState, ProtocolObservation, SessionIdentity,
};
pub use registry::{ProtocolAdapterRegistry, ProtocolAdapterRegistryError};

#[cfg(test)]
mod tests;
