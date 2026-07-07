use std::{num::NonZeroUsize, sync::Arc};

use sql_lens_storage::{ConnectionStore, LiveStatistics, RingBufferStore};
use tokio::sync::RwLock;

use crate::SqlEventBroadcaster;

pub const DEFAULT_EVENT_STORE_CAPACITY: usize = 100_000;
pub const DEFAULT_CONNECTION_STORE_CAPACITY: usize = 10_000;

#[derive(Debug, Clone)]
pub struct ApiState {
    event_store: Arc<RwLock<RingBufferStore>>,
    connection_store: Arc<RwLock<ConnectionStore>>,
    live_statistics: Arc<RwLock<LiveStatistics>>,
    sql_event_broadcaster: SqlEventBroadcaster,
}

impl ApiState {
    pub fn new(event_store: RingBufferStore) -> Self {
        let capacity = NonZeroUsize::new(DEFAULT_CONNECTION_STORE_CAPACITY)
            .expect("default connection store capacity should be non-zero");
        Self::with_stores(event_store, ConnectionStore::new(capacity))
    }

    pub fn with_stores(event_store: RingBufferStore, connection_store: ConnectionStore) -> Self {
        Self::with_all_stores(event_store, connection_store, LiveStatistics::new())
    }

    pub fn with_all_stores(
        event_store: RingBufferStore,
        connection_store: ConnectionStore,
        live_statistics: LiveStatistics,
    ) -> Self {
        Self {
            event_store: Arc::new(RwLock::new(event_store)),
            connection_store: Arc::new(RwLock::new(connection_store)),
            live_statistics: Arc::new(RwLock::new(live_statistics)),
            sql_event_broadcaster: SqlEventBroadcaster::default(),
        }
    }

    pub fn event_store(&self) -> Arc<RwLock<RingBufferStore>> {
        Arc::clone(&self.event_store)
    }

    pub fn connection_store(&self) -> Arc<RwLock<ConnectionStore>> {
        Arc::clone(&self.connection_store)
    }

    pub fn live_statistics(&self) -> Arc<RwLock<LiveStatistics>> {
        Arc::clone(&self.live_statistics)
    }

    pub fn sql_event_broadcaster(&self) -> SqlEventBroadcaster {
        self.sql_event_broadcaster.clone()
    }
}

impl Default for ApiState {
    fn default() -> Self {
        let capacity = NonZeroUsize::new(DEFAULT_EVENT_STORE_CAPACITY)
            .expect("default event store capacity should be non-zero");
        Self::new(RingBufferStore::new(capacity))
    }
}
