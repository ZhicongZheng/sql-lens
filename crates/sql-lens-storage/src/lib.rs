//! Storage backends for SQL Lens.

use sql_lens_core::{SqlEvent, SqlEventId};
use std::{collections::VecDeque, num::NonZeroUsize};

#[derive(Debug, Clone)]
pub struct RingBufferStore {
    capacity: NonZeroUsize,
    events: VecDeque<SqlEvent>,
    total_appended: u64,
    total_evicted: u64,
}

impl RingBufferStore {
    pub fn new(capacity: NonZeroUsize) -> Self {
        Self {
            capacity,
            events: VecDeque::with_capacity(capacity.get()),
            total_appended: 0,
            total_evicted: 0,
        }
    }

    pub fn append(&mut self, event: SqlEvent) -> RingBufferAppendOutcome {
        let stored_event_id = event.id.clone();
        let evicted_event_id = if self.events.len() == self.capacity.get() {
            let evicted = self.events.pop_front();

            if evicted.is_some() {
                self.total_evicted += 1;
            }

            evicted.map(|event| event.id)
        } else {
            None
        };

        self.events.push_back(event);
        self.total_appended += 1;

        RingBufferAppendOutcome {
            stored_event_id,
            evicted_event_id,
        }
    }

    pub fn snapshot(&self) -> Vec<SqlEvent> {
        self.events.iter().cloned().collect()
    }

    pub fn get(&self, id: &SqlEventId) -> Option<&SqlEvent> {
        self.events.iter().find(|event| &event.id == id)
    }

    pub fn stats(&self) -> RingBufferStats {
        RingBufferStats {
            capacity: self.capacity(),
            len: self.len(),
            total_appended: self.total_appended,
            total_evicted: self.total_evicted,
        }
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn capacity(&self) -> usize {
        self.capacity.get()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RingBufferAppendOutcome {
    pub stored_event_id: SqlEventId,
    pub evicted_event_id: Option<SqlEventId>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RingBufferStats {
    pub capacity: usize,
    pub len: usize,
    pub total_appended: u64,
    pub total_evicted: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use sql_lens_core::{
        CaptureStatus, ConnectionId, DatabaseType, DurationMillis, ProtocolMetadata, ProtocolName,
        QueryTiming, SqlEventKind, Timestamp,
    };

    fn capacity(value: usize) -> NonZeroUsize {
        NonZeroUsize::new(value).expect("test capacity should be non-zero")
    }

    fn test_event(id: &str) -> SqlEvent {
        SqlEvent {
            id: SqlEventId(id.to_owned()),
            timestamp: Timestamp("2026-07-06T09:00:00Z".to_owned()),
            protocol: ProtocolName("mysql".to_owned()),
            database_type: DatabaseType("mysql".to_owned()),
            connection_id: ConnectionId("conn_1".to_owned()),
            client_addr: "127.0.0.1:51000".to_owned(),
            backend_addr: "127.0.0.1:3306".to_owned(),
            user: None,
            database: None,
            kind: SqlEventKind::Query,
            status: CaptureStatus::Ok,
            duration: DurationMillis(1),
            original_sql: "SELECT 1".to_owned(),
            normalized_sql: Some("select 1".to_owned()),
            expanded_sql: None,
            fingerprint: Some("select ?".to_owned()),
            parameters: Vec::new(),
            result: None,
            error: None,
            timings: QueryTiming {
                started_at: Timestamp("2026-07-06T09:00:00Z".to_owned()),
                ended_at: Some(Timestamp("2026-07-06T09:00:00Z".to_owned())),
                duration: DurationMillis(1),
            },
            metadata: ProtocolMetadata {
                protocol: ProtocolName("mysql".to_owned()),
                fields: Vec::new(),
            },
        }
    }

    fn event_ids(events: &[SqlEvent]) -> Vec<SqlEventId> {
        events.iter().map(|event| event.id.clone()).collect()
    }

    #[test]
    fn ring_buffer_appends_events() {
        let mut store = RingBufferStore::new(capacity(2));
        let event = test_event("evt_1");

        let outcome = store.append(event.clone());

        assert_eq!(
            outcome,
            RingBufferAppendOutcome {
                stored_event_id: SqlEventId("evt_1".to_owned()),
                evicted_event_id: None,
            }
        );
        assert_eq!(store.len(), 1);
        assert!(!store.is_empty());
        assert_eq!(store.snapshot(), vec![event]);
    }

    #[test]
    fn ring_buffer_enforces_capacity() {
        let mut store = RingBufferStore::new(capacity(2));

        store.append(test_event("evt_1"));
        store.append(test_event("evt_2"));
        store.append(test_event("evt_3"));

        let snapshot = store.snapshot();

        assert_eq!(store.len(), 2);
        assert_eq!(
            event_ids(&snapshot),
            vec![
                SqlEventId("evt_2".to_owned()),
                SqlEventId("evt_3".to_owned())
            ]
        );
    }

    #[test]
    fn ring_buffer_evicts_oldest_event_by_default() {
        let mut store = RingBufferStore::new(capacity(1));

        store.append(test_event("evt_1"));
        let outcome = store.append(test_event("evt_2"));

        assert_eq!(
            outcome,
            RingBufferAppendOutcome {
                stored_event_id: SqlEventId("evt_2".to_owned()),
                evicted_event_id: Some(SqlEventId("evt_1".to_owned())),
            }
        );
        assert_eq!(
            event_ids(&store.snapshot()),
            vec![SqlEventId("evt_2".to_owned())]
        );
    }

    #[test]
    fn ring_buffer_tracks_stats() {
        let mut store = RingBufferStore::new(capacity(2));

        assert_eq!(
            store.stats(),
            RingBufferStats {
                capacity: 2,
                len: 0,
                total_appended: 0,
                total_evicted: 0,
            }
        );

        store.append(test_event("evt_1"));
        store.append(test_event("evt_2"));
        store.append(test_event("evt_3"));

        assert_eq!(
            store.stats(),
            RingBufferStats {
                capacity: 2,
                len: 2,
                total_appended: 3,
                total_evicted: 1,
            }
        );
    }

    #[test]
    fn ring_buffer_requires_non_zero_capacity() {
        assert!(NonZeroUsize::new(0).is_none());
        assert_eq!(RingBufferStore::new(capacity(1)).capacity(), 1);
    }

    #[test]
    fn ring_buffer_gets_existing_event_by_id() {
        let mut store = RingBufferStore::new(capacity(2));
        let first = test_event("evt_1");
        let second = test_event("evt_2");

        store.append(first.clone());
        store.append(second);

        let found = store
            .get(&SqlEventId("evt_1".to_owned()))
            .expect("retained event should be found");

        assert_eq!(found, &first);
    }

    #[test]
    fn ring_buffer_get_returns_none_for_evicted_event() {
        let mut store = RingBufferStore::new(capacity(1));

        store.append(test_event("evt_1"));
        store.append(test_event("evt_2"));

        assert_eq!(store.get(&SqlEventId("evt_1".to_owned())), None);
        assert_eq!(
            store
                .get(&SqlEventId("evt_2".to_owned()))
                .map(|event| event.id.clone()),
            Some(SqlEventId("evt_2".to_owned()))
        );
    }

    #[test]
    fn ring_buffer_get_does_not_mutate_stats() {
        let mut store = RingBufferStore::new(capacity(1));
        store.append(test_event("evt_1"));
        let stats_before = store.stats();

        let _ = store.get(&SqlEventId("evt_1".to_owned()));
        let _ = store.get(&SqlEventId("missing".to_owned()));

        assert_eq!(store.stats(), stats_before);
    }
}
