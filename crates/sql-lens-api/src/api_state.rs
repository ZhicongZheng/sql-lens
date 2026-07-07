use std::{num::NonZeroUsize, sync::Arc};

use sql_lens_storage::RingBufferStore;
use tokio::sync::RwLock;

pub const DEFAULT_EVENT_STORE_CAPACITY: usize = 100_000;

#[derive(Debug, Clone)]
pub struct ApiState {
    event_store: Arc<RwLock<RingBufferStore>>,
}

impl ApiState {
    pub fn new(event_store: RingBufferStore) -> Self {
        Self {
            event_store: Arc::new(RwLock::new(event_store)),
        }
    }

    pub fn event_store(&self) -> Arc<RwLock<RingBufferStore>> {
        Arc::clone(&self.event_store)
    }
}

impl Default for ApiState {
    fn default() -> Self {
        let capacity = NonZeroUsize::new(DEFAULT_EVENT_STORE_CAPACITY)
            .expect("default event store capacity should be non-zero");
        Self::new(RingBufferStore::new(capacity))
    }
}
