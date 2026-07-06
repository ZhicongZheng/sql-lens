use crate::ProtocolAdapter;
use sql_lens_core::ProtocolName;
use std::{collections::HashMap, error::Error, fmt, sync::Arc};

#[derive(Debug, Default)]
pub struct ProtocolAdapterRegistry {
    adapters: HashMap<ProtocolName, Arc<dyn ProtocolAdapter>>,
}

impl ProtocolAdapterRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<A>(&mut self, adapter: A) -> Result<(), ProtocolAdapterRegistryError>
    where
        A: ProtocolAdapter + 'static,
    {
        self.register_shared(Arc::new(adapter))
    }

    pub fn register_shared(
        &mut self,
        adapter: Arc<dyn ProtocolAdapter>,
    ) -> Result<(), ProtocolAdapterRegistryError> {
        let protocol = adapter.protocol_name();

        if self.adapters.contains_key(&protocol) {
            return Err(ProtocolAdapterRegistryError::DuplicateAdapter { protocol });
        }

        self.adapters.insert(protocol, adapter);

        Ok(())
    }

    pub fn resolve(
        &self,
        protocol: &ProtocolName,
    ) -> Result<Arc<dyn ProtocolAdapter>, ProtocolAdapterRegistryError> {
        self.adapters.get(protocol).cloned().ok_or_else(|| {
            ProtocolAdapterRegistryError::UnknownAdapter {
                protocol: protocol.clone(),
            }
        })
    }

    pub fn contains(&self, protocol: &ProtocolName) -> bool {
        self.adapters.contains_key(protocol)
    }

    pub fn len(&self) -> usize {
        self.adapters.len()
    }

    pub fn is_empty(&self) -> bool {
        self.adapters.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolAdapterRegistryError {
    DuplicateAdapter { protocol: ProtocolName },
    UnknownAdapter { protocol: ProtocolName },
}

impl fmt::Display for ProtocolAdapterRegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateAdapter { protocol } => {
                write!(f, "protocol adapter already registered: {}", protocol.0)
            }
            Self::UnknownAdapter { protocol } => {
                write!(f, "unknown protocol adapter: {}", protocol.0)
            }
        }
    }
}

impl Error for ProtocolAdapterRegistryError {}
