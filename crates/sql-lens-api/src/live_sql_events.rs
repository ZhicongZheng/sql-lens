use std::{
    error::Error,
    fmt,
    num::NonZeroUsize,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use sql_lens_core::SqlEvent;
use tokio::sync::broadcast;

pub const DEFAULT_SQL_EVENT_BROADCAST_CAPACITY: usize = 1024;

#[derive(Debug, Clone)]
pub struct SqlEventBroadcaster {
    sender: broadcast::Sender<SqlEvent>,
    counters: Arc<SqlEventBroadcastCounters>,
}

impl SqlEventBroadcaster {
    pub fn new(capacity: NonZeroUsize) -> Self {
        let (sender, _) = broadcast::channel(capacity.get());

        Self {
            sender,
            counters: Arc::new(SqlEventBroadcastCounters::default()),
        }
    }

    pub fn publish(&self, event: SqlEvent) -> SqlEventBroadcastOutcome {
        match self.sender.send(event) {
            Ok(subscriber_count) => {
                self.counters.increment_published_events();
                SqlEventBroadcastOutcome::Delivered { subscriber_count }
            }
            Err(broadcast::error::SendError(_)) => {
                self.counters.increment_no_subscriber_events();
                SqlEventBroadcastOutcome::NoSubscribers
            }
        }
    }

    pub fn subscribe(&self) -> SqlEventSubscription {
        SqlEventSubscription {
            receiver: self.sender.subscribe(),
            counters: Arc::clone(&self.counters),
        }
    }

    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }

    pub fn stats(&self) -> SqlEventBroadcastStats {
        self.counters.stats()
    }
}

impl Default for SqlEventBroadcaster {
    fn default() -> Self {
        let capacity = NonZeroUsize::new(DEFAULT_SQL_EVENT_BROADCAST_CAPACITY)
            .expect("default SQL event broadcast capacity should be non-zero");
        Self::new(capacity)
    }
}

#[derive(Debug)]
pub struct SqlEventSubscription {
    receiver: broadcast::Receiver<SqlEvent>,
    counters: Arc<SqlEventBroadcastCounters>,
}

impl SqlEventSubscription {
    pub async fn recv(&mut self) -> Result<SqlEvent, SqlEventSubscriptionError> {
        match self.receiver.recv().await {
            Ok(event) => Ok(event),
            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                self.counters.increment_lagged_events(skipped);
                Err(SqlEventSubscriptionError::Lagged { skipped })
            }
            Err(broadcast::error::RecvError::Closed) => Err(SqlEventSubscriptionError::Closed),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SqlEventBroadcastOutcome {
    Delivered { subscriber_count: usize },
    NoSubscribers,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SqlEventBroadcastStats {
    pub published_events: u64,
    pub no_subscriber_events: u64,
    pub lagged_events: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SqlEventSubscriptionError {
    Lagged { skipped: u64 },
    Closed,
}

impl fmt::Display for SqlEventSubscriptionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Lagged { skipped } => {
                write!(f, "SQL event subscription lagged by {skipped} events")
            }
            Self::Closed => write!(f, "SQL event subscription broadcaster is closed"),
        }
    }
}

impl Error for SqlEventSubscriptionError {}

#[derive(Debug, Default)]
struct SqlEventBroadcastCounters {
    published_events: AtomicU64,
    no_subscriber_events: AtomicU64,
    lagged_events: AtomicU64,
}

impl SqlEventBroadcastCounters {
    fn increment_published_events(&self) {
        self.published_events.fetch_add(1, Ordering::Relaxed);
    }

    fn increment_no_subscriber_events(&self) {
        self.no_subscriber_events.fetch_add(1, Ordering::Relaxed);
    }

    fn increment_lagged_events(&self, skipped: u64) {
        self.lagged_events.fetch_add(skipped, Ordering::Relaxed);
    }

    fn stats(&self) -> SqlEventBroadcastStats {
        SqlEventBroadcastStats {
            published_events: self.published_events.load(Ordering::Relaxed),
            no_subscriber_events: self.no_subscriber_events.load(Ordering::Relaxed),
            lagged_events: self.lagged_events.load(Ordering::Relaxed),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use crate::test_support::test_event;

    use super::{SqlEventBroadcastOutcome, SqlEventBroadcaster, SqlEventSubscriptionError};

    fn capacity(value: usize) -> NonZeroUsize {
        NonZeroUsize::new(value).expect("test capacity should be non-zero")
    }

    #[test]
    fn publish_without_subscribers_reports_no_subscribers() {
        let broadcaster = SqlEventBroadcaster::new(capacity(1));

        let outcome = broadcaster.publish(test_event("evt_1"));

        assert_eq!(outcome, SqlEventBroadcastOutcome::NoSubscribers);
        assert_eq!(broadcaster.stats().no_subscriber_events, 1);
        assert_eq!(broadcaster.stats().published_events, 0);
    }

    #[tokio::test]
    async fn publish_delivers_event_to_subscriber() {
        let broadcaster = SqlEventBroadcaster::new(capacity(1));
        let mut subscription = broadcaster.subscribe();

        let outcome = broadcaster.publish(test_event("evt_1"));
        let event = subscription.recv().await.expect("event should be received");

        assert_eq!(
            outcome,
            SqlEventBroadcastOutcome::Delivered {
                subscriber_count: 1
            }
        );
        assert_eq!(event.id.0, "evt_1");
        assert_eq!(broadcaster.stats().published_events, 1);
    }

    #[tokio::test]
    async fn lagged_subscription_reports_skipped_events_and_continues() {
        let broadcaster = SqlEventBroadcaster::new(capacity(1));
        let mut subscription = broadcaster.subscribe();

        assert_eq!(
            broadcaster.publish(test_event("evt_1")),
            SqlEventBroadcastOutcome::Delivered {
                subscriber_count: 1
            }
        );
        assert_eq!(
            broadcaster.publish(test_event("evt_2")),
            SqlEventBroadcastOutcome::Delivered {
                subscriber_count: 1
            }
        );

        assert_eq!(
            subscription.recv().await.expect_err("receiver should lag"),
            SqlEventSubscriptionError::Lagged { skipped: 1 }
        );
        assert_eq!(
            subscription
                .recv()
                .await
                .expect("newest retained event should still be readable")
                .id
                .0,
            "evt_2"
        );
        assert_eq!(broadcaster.stats().lagged_events, 1);
    }
}
