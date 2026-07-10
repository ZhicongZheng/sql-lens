use sql_lens_core::SqlEvent;
use std::{
    error::Error,
    fmt,
    num::NonZeroUsize,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};
use tokio::sync::mpsc::{self, error::TrySendError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapturePipelineConfig {
    pub capacity: NonZeroUsize,
    pub overload_policy: CaptureOverloadPolicy,
}

impl CapturePipelineConfig {
    pub fn new(capacity: NonZeroUsize, overload_policy: CaptureOverloadPolicy) -> Self {
        Self {
            capacity,
            overload_policy,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureOverloadPolicy {
    DropNewest,
    RejectNew,
}

#[derive(Debug)]
pub struct CapturePipeline;

impl CapturePipeline {
    pub fn channel(config: CapturePipelineConfig) -> (CaptureEventPublisher, CaptureEventReceiver) {
        let (sender, receiver) = mpsc::channel(config.capacity.get());
        let counters = Arc::new(CapturePipelineCounters::default());

        (
            CaptureEventPublisher {
                sender,
                overload_policy: config.overload_policy,
                counters: Arc::clone(&counters),
            },
            CaptureEventReceiver { receiver, counters },
        )
    }
}

#[derive(Debug, Clone)]
pub struct CaptureEventPublisher {
    sender: mpsc::Sender<SqlEvent>,
    overload_policy: CaptureOverloadPolicy,
    counters: Arc<CapturePipelineCounters>,
}

impl CaptureEventPublisher {
    pub fn publish(&self, event: SqlEvent) -> Result<CapturePublishOutcome, CapturePublishError> {
        match self.sender.try_send(event) {
            Ok(()) => Ok(CapturePublishOutcome::Enqueued),
            Err(TrySendError::Full(event)) => {
                self.counters.increment_dropped_events();

                match self.overload_policy {
                    CaptureOverloadPolicy::DropNewest => Ok(CapturePublishOutcome::Dropped),
                    CaptureOverloadPolicy::RejectNew => Err(CapturePublishError::Full {
                        event: Box::new(event),
                    }),
                }
            }
            Err(TrySendError::Closed(event)) => {
                self.counters.increment_closed_events();
                Err(CapturePublishError::Closed {
                    event: Box::new(event),
                })
            }
        }
    }

    pub fn stats(&self) -> CapturePipelineStats {
        self.counters.stats()
    }
}

#[derive(Debug)]
pub struct CaptureEventReceiver {
    receiver: mpsc::Receiver<SqlEvent>,
    counters: Arc<CapturePipelineCounters>,
}

impl CaptureEventReceiver {
    pub async fn recv(&mut self) -> Option<SqlEvent> {
        self.receiver.recv().await
    }

    pub fn try_recv(&mut self) -> Option<SqlEvent> {
        self.receiver.try_recv().ok()
    }

    pub fn stats(&self) -> CapturePipelineStats {
        self.counters.stats()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapturePublishOutcome {
    Enqueued,
    Dropped,
}

#[derive(Debug, PartialEq)]
pub enum CapturePublishError {
    Full { event: Box<SqlEvent> },
    Closed { event: Box<SqlEvent> },
}

impl CapturePublishError {
    pub fn into_event(self) -> SqlEvent {
        match self {
            Self::Full { event } | Self::Closed { event } => *event,
        }
    }
}

impl fmt::Display for CapturePublishError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Full { .. } => write!(f, "capture event channel is full"),
            Self::Closed { .. } => write!(f, "capture event channel receiver is closed"),
        }
    }
}

impl Error for CapturePublishError {}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CapturePipelineStats {
    pub dropped_events: u64,
    pub closed_events: u64,
}

#[derive(Debug, Default)]
struct CapturePipelineCounters {
    dropped_events: AtomicU64,
    closed_events: AtomicU64,
}

impl CapturePipelineCounters {
    fn increment_dropped_events(&self) {
        self.dropped_events.fetch_add(1, Ordering::Relaxed);
    }

    fn increment_closed_events(&self) {
        self.closed_events.fetch_add(1, Ordering::Relaxed);
    }

    fn stats(&self) -> CapturePipelineStats {
        CapturePipelineStats {
            dropped_events: self.dropped_events.load(Ordering::Relaxed),
            closed_events: self.closed_events.load(Ordering::Relaxed),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sql_lens_core::{
        CaptureStatus, ConnectionId, DatabaseType, DurationMillis, ProtocolMetadata, ProtocolName,
        QueryTiming, SqlEventId, SqlEventKind, Timestamp,
    };
    use std::num::NonZeroUsize;

    fn capacity(value: usize) -> NonZeroUsize {
        NonZeroUsize::new(value).expect("test capacity should be non-zero")
    }

    fn test_event(id: &str) -> SqlEvent {
        SqlEvent {
            id: SqlEventId(id.to_owned()),
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

    #[tokio::test(flavor = "current_thread")]
    async fn publisher_enqueues_event_for_receiver() {
        let config = CapturePipelineConfig::new(capacity(1), CaptureOverloadPolicy::DropNewest);
        let (publisher, mut receiver) = CapturePipeline::channel(config);
        let event = test_event("evt_1");

        let outcome = publisher
            .publish(event.clone())
            .expect("event should enqueue");
        let received = receiver.recv().await.expect("event should be received");

        assert_eq!(outcome, CapturePublishOutcome::Enqueued);
        assert_eq!(received, event);
        assert_eq!(publisher.stats().dropped_events, 0);
        assert_eq!(publisher.stats().closed_events, 0);
        assert_eq!(receiver.stats().dropped_events, 0);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn drop_newest_policy_drops_full_channel_event() {
        let config = CapturePipelineConfig::new(capacity(1), CaptureOverloadPolicy::DropNewest);
        let (publisher, mut receiver) = CapturePipeline::channel(config);
        let first = test_event("evt_1");
        let second = test_event("evt_2");

        assert_eq!(
            publisher
                .publish(first.clone())
                .expect("first event should enqueue"),
            CapturePublishOutcome::Enqueued
        );
        assert_eq!(
            publisher
                .publish(second)
                .expect("second event should be dropped without error"),
            CapturePublishOutcome::Dropped
        );
        assert_eq!(publisher.stats().dropped_events, 1);

        drop(publisher);

        assert_eq!(
            receiver
                .recv()
                .await
                .expect("first event should remain queued"),
            first
        );
        assert_eq!(receiver.recv().await, None);
        assert_eq!(receiver.stats().dropped_events, 1);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn reject_new_policy_returns_full_channel_event() {
        let config = CapturePipelineConfig::new(capacity(1), CaptureOverloadPolicy::RejectNew);
        let (publisher, mut receiver) = CapturePipeline::channel(config);
        let first = test_event("evt_1");
        let second = test_event("evt_2");

        publisher
            .publish(first.clone())
            .expect("first event should enqueue");

        let error = publisher
            .publish(second.clone())
            .expect_err("second event should be rejected");

        assert_eq!(
            error,
            CapturePublishError::Full {
                event: Box::new(second),
            }
        );
        assert_eq!(publisher.stats().dropped_events, 1);

        drop(publisher);

        assert_eq!(
            receiver
                .recv()
                .await
                .expect("first event should remain queued"),
            first
        );
        assert_eq!(receiver.recv().await, None);
    }

    #[test]
    fn publisher_reports_closed_receiver() {
        let config = CapturePipelineConfig::new(capacity(1), CaptureOverloadPolicy::DropNewest);
        let (publisher, receiver) = CapturePipeline::channel(config);
        let event = test_event("evt_1");
        drop(receiver);

        let error = publisher
            .publish(event.clone())
            .expect_err("closed receiver should reject event");

        assert_eq!(
            error,
            CapturePublishError::Closed {
                event: Box::new(event),
            }
        );
        assert_eq!(publisher.stats().dropped_events, 0);
        assert_eq!(publisher.stats().closed_events, 1);
        assert!(!error.to_string().is_empty());
    }
}
