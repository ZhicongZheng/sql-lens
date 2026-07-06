//! TCP proxy runtime for SQL Lens.

mod dialer;
mod forwarding;
mod lifecycle;
mod listener;
mod shutdown;
#[cfg(test)]
mod tests;

pub use dialer::{
    BackendDialConfig, BackendDialError, BackendDialFailure, BackendDialFailureKind, BackendDialer,
    ProxiedConnection,
};
pub use forwarding::{ForwardingError, ForwardingFailure, ForwardingSummary, TcpForwarder};
pub use lifecycle::{
    ConnectionLifecycleFailure, ConnectionLifecycleFailureKind, ConnectionLifecycleIdGenerator,
    ConnectionLifecycleRecord, ConnectionLifecycleTransition,
};
pub use listener::{
    AcceptLoopStats, AcceptedClient, ProxyListenerConfig, ProxyListenerError, TcpProxyListener,
};
pub use shutdown::{
    ActiveSessionDrain, ProxyShutdownConfig, ProxyShutdownError, ProxyShutdownSignal,
    ShutdownDrainSummary,
};
