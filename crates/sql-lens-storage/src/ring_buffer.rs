use sql_lens_core::{
    CaptureStatus, DatabaseType, DurationMillis, ProtocolName, RedactionPolicy, SqlEvent,
    SqlEventId, Timestamp, redact_sql_event,
};
use std::{collections::VecDeque, error::Error, fmt, num::NonZeroUsize};

#[derive(Debug, Clone)]
pub struct RingBufferStore {
    capacity: NonZeroUsize,
    redaction_policy: RedactionPolicy,
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
        Self::with_redaction_policy(capacity, RedactionPolicy::default())
    }

    pub fn with_redaction_policy(
        capacity: NonZeroUsize,
        redaction_policy: RedactionPolicy,
    ) -> Self {
        Self {
            capacity,
            redaction_policy,
            events: VecDeque::with_capacity(capacity.get()),
            next_sequence: 0,
            total_appended: 0,
            total_evicted: 0,
        }
    }

    pub fn append(&mut self, event: SqlEvent) -> RingBufferAppendOutcome {
        let stored_event_id = event.id.clone();
        let event = redact_sql_event(event, &self.redaction_policy);
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

    pub fn query_timeline(
        &self,
        query: RingBufferTimelineQuery,
    ) -> Result<RingBufferTimelinePage, SqlEventFilterError> {
        query.filter.validate()?;

        let before_sequence = query
            .cursor
            .map_or(u64::MAX, |cursor| cursor.before_sequence);
        let limit = query.limit.get();
        let filter = &query.filter;
        let mut events = Vec::with_capacity(limit);
        let mut oldest_returned_sequence = None;
        let mut has_more_older_events = false;

        for entry in self.events.iter().rev() {
            if entry.sequence >= before_sequence {
                continue;
            }

            if !filter.matches(&entry.event) {
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

        Ok(RingBufferTimelinePage {
            events,
            next_cursor,
        })
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
    pub filter: SqlEventFilter,
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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SqlEventFilter {
    pub protocol: Option<ProtocolName>,
    pub database_type: Option<DatabaseType>,
    pub database: Option<String>,
    pub user: Option<String>,
    pub client_addr: Option<String>,
    pub status: Option<CaptureStatus>,
    pub min_duration: Option<DurationMillis>,
    pub max_duration: Option<DurationMillis>,
    pub text: Option<String>,
    pub fingerprint: Option<String>,
    pub from: Option<Timestamp>,
    pub to: Option<Timestamp>,
}

impl SqlEventFilter {
    fn validate(&self) -> Result<(), SqlEventFilterError> {
        if let (Some(min), Some(max)) = (self.min_duration, self.max_duration) {
            if min > max {
                return Err(SqlEventFilterError::InvalidDurationRange { min, max });
            }
        }

        if let (Some(from), Some(to)) = (&self.from, &self.to) {
            if from > to {
                return Err(SqlEventFilterError::InvalidTimestampRange {
                    from: from.clone(),
                    to: to.clone(),
                });
            }
        }

        Ok(())
    }

    fn matches(&self, event: &SqlEvent) -> bool {
        if let Some(protocol) = &self.protocol {
            if &event.protocol != protocol {
                return false;
            }
        }

        if let Some(database_type) = &self.database_type {
            if &event.database_type != database_type {
                return false;
            }
        }

        if let Some(database) = self.database.as_deref() {
            if event.database.as_deref() != Some(database) {
                return false;
            }
        }

        if let Some(user) = self.user.as_deref() {
            if event.user.as_deref() != Some(user) {
                return false;
            }
        }

        if let Some(client_addr) = self.client_addr.as_deref() {
            if event.client_addr != client_addr {
                return false;
            }
        }

        if let Some(status) = self.status {
            if event.status != status {
                return false;
            }
        }

        if let Some(min_duration) = self.min_duration {
            if event.duration < min_duration {
                return false;
            }
        }

        if let Some(max_duration) = self.max_duration {
            if event.duration > max_duration {
                return false;
            }
        }

        if let Some(text) = self.text.as_deref() {
            if !event_text_matches(event, text) {
                return false;
            }
        }

        if let Some(fingerprint) = self.fingerprint.as_deref() {
            if event.fingerprint.as_deref() != Some(fingerprint) {
                return false;
            }
        }

        if let Some(from) = &self.from {
            if &event.timestamp < from {
                return false;
            }
        }

        if let Some(to) = &self.to {
            if &event.timestamp > to {
                return false;
            }
        }

        true
    }
}

fn event_text_matches(event: &SqlEvent, text: &str) -> bool {
    event.original_sql.contains(text)
        || event
            .normalized_sql
            .as_deref()
            .is_some_and(|sql| sql.contains(text))
        || event
            .expanded_sql
            .as_deref()
            .is_some_and(|sql| sql.contains(text))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SqlEventFilterError {
    InvalidDurationRange {
        min: DurationMillis,
        max: DurationMillis,
    },
    InvalidTimestampRange {
        from: Timestamp,
        to: Timestamp,
    },
}

impl fmt::Display for SqlEventFilterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDurationRange { min, max } => {
                write!(
                    f,
                    "invalid duration filter range: min_duration_ms {} is greater than max_duration_ms {}",
                    min.0, max.0
                )
            }
            Self::InvalidTimestampRange { from, to } => {
                write!(
                    f,
                    "invalid timestamp filter range: from {} is greater than to {}",
                    from.0, to.0
                )
            }
        }
    }
}

impl Error for SqlEventFilterError {}

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
        ConnectionId, ProtocolMetadata, QueryTiming, SqlEventKind, SqlParameterValue, Timestamp,
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
        filtered_timeline_query(limit, cursor, SqlEventFilter::default())
    }

    fn filtered_timeline_query(
        limit: usize,
        cursor: Option<RingBufferTimelineCursor>,
        filter: SqlEventFilter,
    ) -> RingBufferTimelineQuery {
        RingBufferTimelineQuery {
            limit: capacity(limit),
            cursor,
            filter,
        }
    }

    fn query_page(
        store: &RingBufferStore,
        query: RingBufferTimelineQuery,
    ) -> RingBufferTimelinePage {
        store
            .query_timeline(query)
            .expect("test timeline query should be valid")
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
    fn ring_buffer_redacts_events_before_retention() {
        let mut store = RingBufferStore::new(capacity(2));
        let mut event = test_event("evt_secret");
        event.parameters.push(sql_lens_core::SqlParameter {
            index: 0,
            name: Some("password".to_owned()),
            value: SqlParameterValue::String("s3cr3t".to_owned()),
            redacted: false,
        });
        event.original_sql = "SELECT * FROM users WHERE password = ?".to_owned();
        event.expanded_sql = Some("SELECT * FROM users WHERE password = 's3cr3t'".to_owned());

        store.append(event);
        let retained = store
            .get(&SqlEventId("evt_secret".to_owned()))
            .expect("retained event should be found");

        assert!(retained.parameters[0].redacted);
        assert_eq!(
            retained.parameters[0].value,
            SqlParameterValue::String("***".to_owned())
        );
        assert_eq!(
            retained.expanded_sql.as_deref(),
            Some("SELECT * FROM users WHERE password = '***'")
        );
        assert!(
            !retained
                .expanded_sql
                .as_deref()
                .expect("expanded SQL should be present")
                .contains("s3cr3t")
        );
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

        let page = query_page(&store, timeline_query(3, None));

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

        let page = query_page(&store, timeline_query(2, None));

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

        let first_page = query_page(&store, timeline_query(2, None));
        let second_page = query_page(&store, timeline_query(2, first_page.next_cursor));
        let third_page = query_page(&store, timeline_query(2, second_page.next_cursor));

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

        let first_page = query_page(&store, timeline_query(2, None));
        store.append(test_event("evt_5"));
        let second_page = query_page(&store, timeline_query(2, first_page.next_cursor));

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
    fn ring_buffer_timeline_filters_by_protocol_and_status() {
        let mut store = RingBufferStore::new(capacity(3));
        let mut mysql_error = test_event("evt_1");
        mysql_error.status = CaptureStatus::Error;
        let mut postgresql_error = test_event("evt_2");
        postgresql_error.protocol = ProtocolName("postgresql".to_owned());
        postgresql_error.status = CaptureStatus::Error;
        let mysql_ok = test_event("evt_3");

        store.append(mysql_error);
        store.append(postgresql_error);
        store.append(mysql_ok);

        let page = query_page(
            &store,
            filtered_timeline_query(
                10,
                None,
                SqlEventFilter {
                    protocol: Some(ProtocolName("mysql".to_owned())),
                    status: Some(CaptureStatus::Error),
                    ..SqlEventFilter::default()
                },
            ),
        );

        assert_eq!(
            event_ids(&page.events),
            vec![SqlEventId("evt_1".to_owned())]
        );
        assert_eq!(page.next_cursor, None);
    }

    #[test]
    fn ring_buffer_timeline_filters_by_database_type_database_and_user() {
        let mut store = RingBufferStore::new(capacity(3));
        let mut target = test_event("evt_1");
        target.database_type = DatabaseType("starrocks".to_owned());
        target.database = Some("analytics".to_owned());
        target.user = Some("analyst".to_owned());
        let mut wrong_user = target.clone();
        wrong_user.id = SqlEventId("evt_2".to_owned());
        wrong_user.user = Some("app".to_owned());
        let mut wrong_database = target.clone();
        wrong_database.id = SqlEventId("evt_3".to_owned());
        wrong_database.database = Some("ops".to_owned());

        store.append(target);
        store.append(wrong_user);
        store.append(wrong_database);

        let page = query_page(
            &store,
            filtered_timeline_query(
                10,
                None,
                SqlEventFilter {
                    database_type: Some(DatabaseType("starrocks".to_owned())),
                    database: Some("analytics".to_owned()),
                    user: Some("analyst".to_owned()),
                    ..SqlEventFilter::default()
                },
            ),
        );

        assert_eq!(
            event_ids(&page.events),
            vec![SqlEventId("evt_1".to_owned())]
        );
        assert_eq!(page.next_cursor, None);
    }

    #[test]
    fn ring_buffer_timeline_filters_by_duration_range() {
        let mut store = RingBufferStore::new(capacity(3));
        let mut fast = test_event("evt_1");
        fast.duration = DurationMillis(1);
        let mut target = test_event("evt_2");
        target.duration = DurationMillis(5);
        let mut slow = test_event("evt_3");
        slow.duration = DurationMillis(10);

        store.append(fast);
        store.append(target);
        store.append(slow);

        let page = query_page(
            &store,
            filtered_timeline_query(
                10,
                None,
                SqlEventFilter {
                    min_duration: Some(DurationMillis(2)),
                    max_duration: Some(DurationMillis(8)),
                    ..SqlEventFilter::default()
                },
            ),
        );

        assert_eq!(
            event_ids(&page.events),
            vec![SqlEventId("evt_2".to_owned())]
        );
        assert_eq!(page.next_cursor, None);
    }

    #[test]
    fn ring_buffer_timeline_filters_by_sql_text() {
        let mut store = RingBufferStore::new(capacity(3));
        let mut original_match = test_event("evt_1");
        original_match.original_sql = "SELECT * FROM orders".to_owned();
        original_match.normalized_sql = None;
        original_match.expanded_sql = None;
        let mut normalized_match = test_event("evt_2");
        normalized_match.original_sql = "SELECT * FROM invoices WHERE id = ?".to_owned();
        normalized_match.normalized_sql = Some("select * from orders where id = ?".to_owned());
        normalized_match.expanded_sql = None;
        let mut expanded_match = test_event("evt_3");
        expanded_match.original_sql = "SELECT * FROM invoices WHERE id = ?".to_owned();
        expanded_match.normalized_sql = None;
        expanded_match.expanded_sql = Some("SELECT * FROM orders WHERE id = 42".to_owned());

        store.append(original_match);
        store.append(normalized_match);
        store.append(expanded_match);

        let page = query_page(
            &store,
            filtered_timeline_query(
                10,
                None,
                SqlEventFilter {
                    text: Some("orders".to_owned()),
                    ..SqlEventFilter::default()
                },
            ),
        );

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
    fn ring_buffer_timeline_filters_by_client_addr_and_fingerprint() {
        let mut store = RingBufferStore::new(capacity(3));
        let mut target = test_event("evt_1");
        target.client_addr = "127.0.0.1:51000".to_owned();
        target.fingerprint = Some("select * from users where id = ?".to_owned());
        let mut wrong_client = target.clone();
        wrong_client.id = SqlEventId("evt_2".to_owned());
        wrong_client.client_addr = "127.0.0.1:51001".to_owned();
        let mut wrong_fingerprint = target.clone();
        wrong_fingerprint.id = SqlEventId("evt_3".to_owned());
        wrong_fingerprint.fingerprint = Some("select * from orders where id = ?".to_owned());

        store.append(target);
        store.append(wrong_client);
        store.append(wrong_fingerprint);

        let page = query_page(
            &store,
            filtered_timeline_query(
                10,
                None,
                SqlEventFilter {
                    client_addr: Some("127.0.0.1:51000".to_owned()),
                    fingerprint: Some("select * from users where id = ?".to_owned()),
                    ..SqlEventFilter::default()
                },
            ),
        );

        assert_eq!(
            event_ids(&page.events),
            vec![SqlEventId("evt_1".to_owned())]
        );
        assert_eq!(page.next_cursor, None);
    }

    #[test]
    fn ring_buffer_timeline_filters_by_timestamp_range() {
        let mut store = RingBufferStore::new(capacity(3));
        let mut before = test_event("evt_1");
        before.timestamp = Timestamp("2026-07-06T09:00:00Z".to_owned());
        let mut target = test_event("evt_2");
        target.timestamp = Timestamp("2026-07-06T09:05:00Z".to_owned());
        let mut after = test_event("evt_3");
        after.timestamp = Timestamp("2026-07-06T09:10:00Z".to_owned());

        store.append(before);
        store.append(target);
        store.append(after);

        let page = query_page(
            &store,
            filtered_timeline_query(
                10,
                None,
                SqlEventFilter {
                    from: Some(Timestamp("2026-07-06T09:01:00Z".to_owned())),
                    to: Some(Timestamp("2026-07-06T09:09:00Z".to_owned())),
                    ..SqlEventFilter::default()
                },
            ),
        );

        assert_eq!(
            event_ids(&page.events),
            vec![SqlEventId("evt_2".to_owned())]
        );
        assert_eq!(page.next_cursor, None);
    }

    #[test]
    fn ring_buffer_timeline_filtered_cursor_pages_matching_events_only() {
        let mut store = RingBufferStore::new(capacity(6));
        let mut first_error = test_event("evt_1");
        first_error.status = CaptureStatus::Error;
        let ok_after_first = test_event("evt_2");
        let mut second_error = test_event("evt_3");
        second_error.status = CaptureStatus::Error;
        let ok_after_second = test_event("evt_4");
        let mut third_error = test_event("evt_5");
        third_error.status = CaptureStatus::Error;

        store.append(first_error);
        store.append(ok_after_first);
        store.append(second_error);
        store.append(ok_after_second);
        store.append(third_error);

        let error_filter = SqlEventFilter {
            status: Some(CaptureStatus::Error),
            ..SqlEventFilter::default()
        };
        let first_page = query_page(
            &store,
            filtered_timeline_query(1, None, error_filter.clone()),
        );
        let mut newer_error = test_event("evt_6");
        newer_error.status = CaptureStatus::Error;
        store.append(newer_error);
        let second_page = query_page(
            &store,
            filtered_timeline_query(1, first_page.next_cursor, error_filter.clone()),
        );
        let third_page = query_page(
            &store,
            filtered_timeline_query(1, second_page.next_cursor, error_filter),
        );

        assert_eq!(
            event_ids(&first_page.events),
            vec![SqlEventId("evt_5".to_owned())]
        );
        assert_eq!(
            event_ids(&second_page.events),
            vec![SqlEventId("evt_3".to_owned())]
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
    fn ring_buffer_timeline_rejects_invalid_duration_range() {
        let store = RingBufferStore::new(capacity(1));

        let error = store
            .query_timeline(filtered_timeline_query(
                1,
                None,
                SqlEventFilter {
                    min_duration: Some(DurationMillis(10)),
                    max_duration: Some(DurationMillis(5)),
                    ..SqlEventFilter::default()
                },
            ))
            .expect_err("invalid duration range should fail");

        assert_eq!(
            error,
            SqlEventFilterError::InvalidDurationRange {
                min: DurationMillis(10),
                max: DurationMillis(5),
            }
        );
        assert!(!error.to_string().is_empty());
    }

    #[test]
    fn ring_buffer_timeline_rejects_invalid_timestamp_range() {
        let store = RingBufferStore::new(capacity(1));

        let error = store
            .query_timeline(filtered_timeline_query(
                1,
                None,
                SqlEventFilter {
                    from: Some(Timestamp("2026-07-06T09:10:00Z".to_owned())),
                    to: Some(Timestamp("2026-07-06T09:00:00Z".to_owned())),
                    ..SqlEventFilter::default()
                },
            ))
            .expect_err("invalid timestamp range should fail");

        assert_eq!(
            error,
            SqlEventFilterError::InvalidTimestampRange {
                from: Timestamp("2026-07-06T09:10:00Z".to_owned()),
                to: Timestamp("2026-07-06T09:00:00Z".to_owned()),
            }
        );
        assert!(!error.to_string().is_empty());
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
