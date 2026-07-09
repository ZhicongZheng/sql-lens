use sql_lens_core::{CaptureStatus, DurationMillis, SqlEvent};

pub const DEFAULT_SLOW_THRESHOLD_MS: u64 = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlowQueryClassifier {
    threshold: DurationMillis,
}

impl SlowQueryClassifier {
    pub fn new(threshold: DurationMillis) -> Self {
        Self { threshold }
    }

    pub fn threshold(&self) -> DurationMillis {
        self.threshold
    }

    pub fn classify(&self, mut event: SqlEvent) -> SqlEvent {
        if event.status == CaptureStatus::Ok && event.duration.0 >= self.threshold.0 {
            event.status = CaptureStatus::Slow;
        }

        event
    }
}

impl Default for SlowQueryClassifier {
    fn default() -> Self {
        Self::new(DurationMillis(DEFAULT_SLOW_THRESHOLD_MS))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sql_lens_core::{
        ConnectionId, DatabaseType, ProtocolMetadata, ProtocolName, QueryTiming, SqlEventId,
        SqlEventKind, Timestamp,
    };

    #[test]
    fn below_threshold_ok_event_remains_ok() {
        let classifier = SlowQueryClassifier::new(DurationMillis(100));
        let event = test_event(CaptureStatus::Ok, DurationMillis(99));

        let classified = classifier.classify(event);

        assert_eq!(classified.status, CaptureStatus::Ok);
    }

    #[test]
    fn at_threshold_ok_event_becomes_slow() {
        let classifier = SlowQueryClassifier::new(DurationMillis(100));
        let event = test_event(CaptureStatus::Ok, DurationMillis(100));

        let classified = classifier.classify(event);

        assert_eq!(classified.status, CaptureStatus::Slow);
    }

    #[test]
    fn above_threshold_ok_event_becomes_slow() {
        let classifier = SlowQueryClassifier::new(DurationMillis(100));
        let event = test_event(CaptureStatus::Ok, DurationMillis(101));

        let classified = classifier.classify(event);

        assert_eq!(classified.status, CaptureStatus::Slow);
    }

    #[test]
    fn non_ok_statuses_are_not_overwritten() {
        let classifier = SlowQueryClassifier::new(DurationMillis(100));

        for status in [
            CaptureStatus::Slow,
            CaptureStatus::Error,
            CaptureStatus::Unknown,
        ] {
            let event = test_event(status, DurationMillis(1_000));
            let classified = classifier.classify(event);

            assert_eq!(classified.status, status);
        }
    }

    #[test]
    fn default_threshold_is_documented_global_default() {
        let classifier = SlowQueryClassifier::default();

        assert_eq!(
            classifier.threshold(),
            DurationMillis(DEFAULT_SLOW_THRESHOLD_MS)
        );
    }

    fn test_event(status: CaptureStatus, duration: DurationMillis) -> SqlEvent {
        SqlEvent {
            id: SqlEventId("evt_1".to_owned()),
            timestamp: Timestamp("2026-07-06T09:00:00Z".to_owned()),
            target_name: Some("mysql-local".to_owned()),
            protocol: ProtocolName("mysql".to_owned()),
            database_type: DatabaseType("mysql".to_owned()),
            connection_id: ConnectionId("conn_1".to_owned()),
            client_addr: "127.0.0.1:51000".to_owned(),
            backend_addr: "127.0.0.1:3306".to_owned(),
            user: None,
            database: None,
            kind: SqlEventKind::Query,
            status,
            duration,
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
                duration,
            },
            metadata: ProtocolMetadata {
                protocol: ProtocolName("mysql".to_owned()),
                fields: Vec::new(),
            },
        }
    }
}
