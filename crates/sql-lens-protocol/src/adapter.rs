use sql_lens_core::{ConnectionInfo, ProtocolName, SqlEvent};
use std::{any::Any, error::Error, fmt};

pub trait ProtocolAdapter: fmt::Debug + Send + Sync {
    fn protocol_name(&self) -> ProtocolName;

    fn create_connection_state(
        &self,
        context: &ProtocolConnectionContext,
    ) -> Box<dyn ProtocolConnectionState>;

    fn observe_client_bytes(
        &self,
        state: &mut dyn ProtocolConnectionState,
        bytes: &[u8],
        events: &mut dyn CaptureEventEmitter,
    ) -> Result<ProtocolObservation, ProtocolAdapterError>;

    fn observe_backend_bytes(
        &self,
        state: &mut dyn ProtocolConnectionState,
        bytes: &[u8],
        events: &mut dyn CaptureEventEmitter,
    ) -> Result<ProtocolObservation, ProtocolAdapterError>;
}

pub trait ProtocolConnectionState: Any + fmt::Debug + Send {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T> ProtocolConnectionState for T
where
    T: Any + fmt::Debug + Send,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolConnectionContext {
    pub connection: ConnectionInfo,
}

impl ProtocolConnectionContext {
    pub fn new(connection: ConnectionInfo) -> Self {
        Self { connection }
    }
}

pub trait CaptureEventEmitter {
    fn emit(&mut self, event: SqlEvent);
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProtocolObservation {
    pub bytes_observed: usize,
    pub events_emitted: usize,
}

impl ProtocolObservation {
    pub fn new(bytes_observed: usize, events_emitted: usize) -> Self {
        Self {
            bytes_observed,
            events_emitted,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolAdapterError {
    InvalidConnectionState { expected: &'static str },
    ObservationFailed { message: String },
}

impl fmt::Display for ProtocolAdapterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConnectionState { expected } => {
                write!(f, "invalid protocol connection state, expected {expected}")
            }
            Self::ObservationFailed { message } => {
                write!(f, "protocol observation failed: {message}")
            }
        }
    }
}

impl Error for ProtocolAdapterError {}
