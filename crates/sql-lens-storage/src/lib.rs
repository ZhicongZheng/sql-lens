//! Storage backends for SQL Lens.

use sql_lens_core::{SqlEvent, SqlEventId};
use std::{collections::VecDeque, num::NonZeroUsize};

#[derive(Debug, Clone)]
pub struct RingBufferStore {
    capacity: NonZeroUsize,
    events: VecDeque<RingBufferEntry>,
    next_sequence: u64,
    total_appended: u64,
    total_evicted: u64,
}

#[derive(Debug, Clone)]
struct RingBufferEntry {
    sequence: u64,
    event: SqlEvent,
}

impl RingBufferStore {
    pub fn new(capacity: NonZeroUsize) -> Self {
        Self {
            capacity,
            events: VecDeque::with_capacity(capacity.get()),
            next_sequence: 0,
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

            evicted.map(|entry| entry.event.id)
        } else {
            None
        };

        let sequence = self.next_sequence;
        self.next_sequence += 1;
        self.events.push_back(RingBufferEntry { sequence, event });
        self.total_appended += 1;

        RingBufferAppendOutcome {
            stored_event_id,
            evicted_event_id,
        }
    }

    pub fn snapshot(&self) -> Vec<SqlEvent> {
        self.events
            .iter()
            .map(|entry| entry.event.clone())
            .collect()
    }

    pub fn get(&self, id: &SqlEventId) -> Option<&SqlEvent> {
        self.events
            .iter()
            .find(|entry| &entry.event.id == id)
            .map(|entry| &entry.event)
    }

    pub fn query_timeline(&self, query: RingBufferTimelineQuery) -> RingBufferTimelinePage {
        let before_sequence = query
            .cursor
            .map_or(u64::MAX, |cursor| cursor.before_sequence);
        let limit = query.limit.get();
        let mut events = Vec::with_capacity(limit);
        let mut oldest_returned_sequence = None;
        let mut has_more_older_events = false;

        for entry in self.events.iter().rev() {
            if entry.sequence >= before_sequence {
                continue;
            }

            if events.len() == limit {
                has_more_older_events = true;
                break;
            }

            oldest_returned_sequence = Some(entry.sequence);
            events.push(entry.event.clone());
        }

        let next_cursor = if has_more_older_events {
            oldest_returned_sequence
                .map(|before_sequence| RingBufferTimelineCursor { before_sequence })
        } else {
            None
        };

        RingBufferTimelinePage {
            events,
            next_cursor,
        }
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RingBufferTimelineQuery {
    pub limit: NonZeroUsize,
    pub cursor: Option<RingBufferTimelineCursor>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RingBufferTimelineCursor {
    pub before_sequence: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RingBufferTimelinePage {
    pub events: Vec<SqlEvent>,
    pub next_cursor: Option<RingBufferTimelineCursor>,
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

    fn timeline_query(
        limit: usize,
        cursor: Option<RingBufferTimelineCursor>,
    ) -> RingBufferTimelineQuery {
        RingBufferTimelineQuery {
            limit: capacity(limit),
            cursor,
        }
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

    #[test]
    fn ring_buffer_timeline_returns_newest_events_first() {
        let mut store = RingBufferStore::new(capacity(3));

        store.append(test_event("evt_1"));
        store.append(test_event("evt_2"));
        store.append(test_event("evt_3"));

        let page = store.query_timeline(timeline_query(3, None));

        assert_eq!(
            event_ids(&page.events),
            vec![
                SqlEventId("evt_3".to_owned()),
                SqlEventId("evt_2".to_owned()),
                SqlEventId("evt_1".to_owned())
            ]
        );
        assert_eq!(page.next_cursor, None);
    }

    #[test]
    fn ring_buffer_timeline_limit_truncates_results() {
        let mut store = RingBufferStore::new(capacity(3));

        store.append(test_event("evt_1"));
        store.append(test_event("evt_2"));
        store.append(test_event("evt_3"));

        let page = store.query_timeline(timeline_query(2, None));

        assert_eq!(
            event_ids(&page.events),
            vec![
                SqlEventId("evt_3".to_owned()),
                SqlEventId("evt_2".to_owned())
            ]
        );
        assert!(page.next_cursor.is_some());
    }

    #[test]
    fn ring_buffer_timeline_cursor_pages_older_events_without_duplicates() {
        let mut store = RingBufferStore::new(capacity(5));

        store.append(test_event("evt_1"));
        store.append(test_event("evt_2"));
        store.append(test_event("evt_3"));
        store.append(test_event("evt_4"));
        store.append(test_event("evt_5"));

        let first_page = store.query_timeline(timeline_query(2, None));
        let second_page = store.query_timeline(timeline_query(2, first_page.next_cursor));
        let third_page = store.query_timeline(timeline_query(2, second_page.next_cursor));

        assert_eq!(
            event_ids(&first_page.events),
            vec![
                SqlEventId("evt_5".to_owned()),
                SqlEventId("evt_4".to_owned())
            ]
        );
        assert_eq!(
            event_ids(&second_page.events),
            vec![
                SqlEventId("evt_3".to_owned()),
                SqlEventId("evt_2".to_owned())
            ]
        );
        assert_eq!(
            event_ids(&third_page.events),
            vec![SqlEventId("evt_1".to_owned())]
        );
        assert!(first_page.next_cursor.is_some());
        assert!(second_page.next_cursor.is_some());
        assert_eq!(third_page.next_cursor, None);
    }

    #[test]
    fn ring_buffer_timeline_cursor_is_stable_after_newer_append() {
        let mut store = RingBufferStore::new(capacity(5));

        store.append(test_event("evt_1"));
        store.append(test_event("evt_2"));
        store.append(test_event("evt_3"));
        store.append(test_event("evt_4"));

        let first_page = store.query_timeline(timeline_query(2, None));
        store.append(test_event("evt_5"));
        let second_page = store.query_timeline(timeline_query(2, first_page.next_cursor));

        assert_eq!(
            event_ids(&first_page.events),
            vec![
                SqlEventId("evt_4".to_owned()),
                SqlEventId("evt_3".to_owned())
            ]
        );
        assert_eq!(
            event_ids(&second_page.events),
            vec![
                SqlEventId("evt_2".to_owned()),
                SqlEventId("evt_1".to_owned())
            ]
        );
        assert_eq!(second_page.next_cursor, None);
    }

    #[test]
    fn ring_buffer_snapshot_remains_oldest_to_newest() {
        let mut store = RingBufferStore::new(capacity(3));

        store.append(test_event("evt_1"));
        store.append(test_event("evt_2"));
        store.append(test_event("evt_3"));

        assert_eq!(
            event_ids(&store.snapshot()),
            vec![
                SqlEventId("evt_1".to_owned()),
                SqlEventId("evt_2".to_owned()),
                SqlEventId("evt_3".to_owned())
            ]
        );
    }
}
