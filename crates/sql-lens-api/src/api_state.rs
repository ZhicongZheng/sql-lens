use std::{num::NonZeroUsize, sync::Arc};

use sql_lens_core::RedactionPolicy;
use sql_lens_storage::{ConnectionStore, LiveStatistics, RingBufferStore, SqliteEventStore};
use tokio::sync::RwLock;

use crate::{SqlEventBroadcaster, event_reader::SqlEventReadStore};

pub const DEFAULT_EVENT_STORE_CAPACITY: usize = 100_000;
pub const DEFAULT_CONNECTION_STORE_CAPACITY: usize = 10_000;

#[derive(Debug, Clone)]
pub struct ApiState {
    event_store: Arc<RwLock<RingBufferStore>>,
    event_reader: SqlEventReadStore,
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

    pub fn with_redaction_policy(
        event_store: RingBufferStore,
        redaction_policy: RedactionPolicy,
    ) -> Self {
        let capacity = NonZeroUsize::new(DEFAULT_CONNECTION_STORE_CAPACITY)
            .expect("default connection store capacity should be non-zero");

        Self::with_all_stores_and_redaction(
            event_store,
            ConnectionStore::new(capacity),
            LiveStatistics::new(),
            redaction_policy,
        )
    }

    pub fn with_all_stores(
        event_store: RingBufferStore,
        connection_store: ConnectionStore,
        live_statistics: LiveStatistics,
    ) -> Self {
        Self::with_all_stores_and_redaction(
            event_store,
            connection_store,
            live_statistics,
            RedactionPolicy::default(),
        )
    }

    pub fn with_all_stores_and_redaction(
        event_store: RingBufferStore,
        connection_store: ConnectionStore,
        live_statistics: LiveStatistics,
        redaction_policy: RedactionPolicy,
    ) -> Self {
        let event_store = Arc::new(RwLock::new(event_store));
        let broadcaster_capacity =
            NonZeroUsize::new(crate::live_sql_events::DEFAULT_SQL_EVENT_BROADCAST_CAPACITY)
                .expect("default SQL event broadcast capacity should be non-zero");

        Self {
            event_reader: SqlEventReadStore::ring_buffer(Arc::clone(&event_store)),
            event_store,
            connection_store: Arc::new(RwLock::new(connection_store)),
            live_statistics: Arc::new(RwLock::new(live_statistics)),
            sql_event_broadcaster: SqlEventBroadcaster::with_redaction_policy(
                broadcaster_capacity,
                redaction_policy,
            ),
        }
    }

    pub fn with_sqlite_event_reader(
        event_store: RingBufferStore,
        sqlite_store: SqliteEventStore,
    ) -> Self {
        let capacity = NonZeroUsize::new(DEFAULT_CONNECTION_STORE_CAPACITY)
            .expect("default connection store capacity should be non-zero");

        Self::with_all_stores_and_sqlite_event_reader(
            event_store,
            ConnectionStore::new(capacity),
            LiveStatistics::new(),
            sqlite_store,
        )
    }

    pub fn with_sqlite_event_reader_and_redaction(
        event_store: RingBufferStore,
        sqlite_store: SqliteEventStore,
        redaction_policy: RedactionPolicy,
    ) -> Self {
        let capacity = NonZeroUsize::new(DEFAULT_CONNECTION_STORE_CAPACITY)
            .expect("default connection store capacity should be non-zero");

        Self::with_all_stores_and_sqlite_event_reader_and_redaction(
            event_store,
            ConnectionStore::new(capacity),
            LiveStatistics::new(),
            sqlite_store,
            redaction_policy,
        )
    }

    pub fn with_all_stores_and_sqlite_event_reader(
        event_store: RingBufferStore,
        connection_store: ConnectionStore,
        live_statistics: LiveStatistics,
        sqlite_store: SqliteEventStore,
    ) -> Self {
        Self::with_all_stores_and_sqlite_event_reader_and_redaction(
            event_store,
            connection_store,
            live_statistics,
            sqlite_store,
            RedactionPolicy::default(),
        )
    }

    pub fn with_all_stores_and_sqlite_event_reader_and_redaction(
        event_store: RingBufferStore,
        connection_store: ConnectionStore,
        live_statistics: LiveStatistics,
        sqlite_store: SqliteEventStore,
        redaction_policy: RedactionPolicy,
    ) -> Self {
        let event_store = Arc::new(RwLock::new(event_store));
        let broadcaster_capacity =
            NonZeroUsize::new(crate::live_sql_events::DEFAULT_SQL_EVENT_BROADCAST_CAPACITY)
                .expect("default SQL event broadcast capacity should be non-zero");

        Self {
            event_reader: SqlEventReadStore::sqlite(sqlite_store),
            event_store,
            connection_store: Arc::new(RwLock::new(connection_store)),
            live_statistics: Arc::new(RwLock::new(live_statistics)),
            sql_event_broadcaster: SqlEventBroadcaster::with_redaction_policy(
                broadcaster_capacity,
                redaction_policy,
            ),
        }
    }

    pub fn event_store(&self) -> Arc<RwLock<RingBufferStore>> {
        Arc::clone(&self.event_store)
    }

    pub(crate) fn event_reader(&self) -> SqlEventReadStore {
        self.event_reader.clone()
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
