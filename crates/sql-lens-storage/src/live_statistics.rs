use sql_lens_core::{CaptureStatus, ConnectionId, DurationMillis, SqlEvent};
use std::{
    collections::{HashSet, VecDeque},
    time::{Duration, Instant},
};

const LIVE_QPS_WINDOW: Duration = Duration::from_secs(60);
const LATENCY_BUCKET_UPPER_BOUNDS_MS: [u64; 8] = [1, 5, 10, 50, 100, 500, 1000, 5000];

#[derive(Debug, Clone)]
pub struct LiveStatistics {
    total_events: u64,
    error_events: u64,
    slow_events: u64,
    latency_bucket_counts: Vec<u64>,
    recent_event_times: VecDeque<Instant>,
    recent_latency_samples: VecDeque<LatencySample>,
    active_connections: HashSet<ConnectionId>,
}

impl LiveStatistics {
    pub fn new() -> Self {
        Self {
            total_events: 0,
            error_events: 0,
            slow_events: 0,
            latency_bucket_counts: vec![0; LATENCY_BUCKET_UPPER_BOUNDS_MS.len() + 1],
            recent_event_times: VecDeque::new(),
            recent_latency_samples: VecDeque::new(),
            active_connections: HashSet::new(),
        }
    }

    pub fn record_sql_event(&mut self, event: &SqlEvent) {
        self.record_sql_event_at(event, Instant::now());
    }

    pub fn record_sql_event_at(&mut self, event: &SqlEvent, recorded_at: Instant) {
        self.prune_recent_events(recorded_at);

        self.total_events += 1;
        match event.status {
            CaptureStatus::Error => self.error_events += 1,
            CaptureStatus::Slow => self.slow_events += 1,
            CaptureStatus::Ok | CaptureStatus::Unknown => {}
        }

        let bucket_index = latency_bucket_index(event.duration);
        self.latency_bucket_counts[bucket_index] += 1;
        self.recent_event_times.push_back(recorded_at);
        self.recent_latency_samples.push_back(LatencySample {
            recorded_at,
            duration: event.duration,
        });
    }

    pub fn record_connection_opened(&mut self, connection_id: ConnectionId) {
        self.active_connections.insert(connection_id);
    }

    pub fn record_connection_closed(&mut self, connection_id: &ConnectionId) {
        self.active_connections.remove(connection_id);
    }

    pub fn snapshot(&mut self) -> LiveStatisticsSnapshot {
        self.snapshot_at(Instant::now())
    }

    pub fn snapshot_at(&mut self, now: Instant) -> LiveStatisticsSnapshot {
        self.prune_recent_events(now);

        LiveStatisticsSnapshot {
            total_events: self.total_events,
            error_events: self.error_events,
            slow_events: self.slow_events,
            qps_window_secs: LIVE_QPS_WINDOW.as_secs(),
            qps: self.recent_event_times.len() as f64 / LIVE_QPS_WINDOW.as_secs_f64(),
            latency_buckets: latency_bucket_counts(&self.latency_bucket_counts),
            latency_percentiles: latency_percentiles(&self.recent_latency_samples),
            active_connections: self.active_connections.len(),
        }
    }

    fn prune_recent_events(&mut self, now: Instant) {
        while self.recent_event_times.front().is_some_and(|recorded_at| {
            now.saturating_duration_since(*recorded_at) > LIVE_QPS_WINDOW
        }) {
            self.recent_event_times.pop_front();
        }

        while self.recent_latency_samples.front().is_some_and(|sample| {
            now.saturating_duration_since(sample.recorded_at) > LIVE_QPS_WINDOW
        }) {
            self.recent_latency_samples.pop_front();
        }
    }
}

impl Default for LiveStatistics {
    fn default() -> Self {
        Self::new()
    }
}

fn latency_bucket_index(duration: DurationMillis) -> usize {
    LATENCY_BUCKET_UPPER_BOUNDS_MS
        .iter()
        .position(|upper_bound| duration.0 <= *upper_bound)
        .unwrap_or(LATENCY_BUCKET_UPPER_BOUNDS_MS.len())
}

fn latency_bucket_counts(counts: &[u64]) -> Vec<LatencyBucketCount> {
    LATENCY_BUCKET_UPPER_BOUNDS_MS
        .iter()
        .enumerate()
        .map(|(index, upper_bound)| LatencyBucketCount {
            upper_bound: Some(DurationMillis(*upper_bound)),
            count: counts[index],
        })
        .chain(std::iter::once(LatencyBucketCount {
            upper_bound: None,
            count: counts[LATENCY_BUCKET_UPPER_BOUNDS_MS.len()],
        }))
        .collect()
}

fn latency_percentiles(samples: &VecDeque<LatencySample>) -> LatencyPercentiles {
    if samples.is_empty() {
        return LatencyPercentiles {
            p50: 0.0,
            p95: 0.0,
            p99: 0.0,
        };
    }

    let mut durations = samples
        .iter()
        .map(|sample| sample.duration.0)
        .collect::<Vec<_>>();
    durations.sort_unstable();

    LatencyPercentiles {
        p50: percentile(&durations, 50),
        p95: percentile(&durations, 95),
        p99: percentile(&durations, 99),
    }
}

fn percentile(sorted_values: &[u64], percentile: u64) -> f64 {
    debug_assert!(!sorted_values.is_empty());
    debug_assert!(percentile > 0);
    debug_assert!(percentile <= 100);

    let rank = (sorted_values.len() as u64 * percentile).div_ceil(100);
    let index = rank.saturating_sub(1) as usize;
    sorted_values[index] as f64
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LatencySample {
    recorded_at: Instant,
    duration: DurationMillis,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LiveStatisticsSnapshot {
    pub total_events: u64,
    pub error_events: u64,
    pub slow_events: u64,
    pub qps_window_secs: u64,
    pub qps: f64,
    pub latency_buckets: Vec<LatencyBucketCount>,
    pub latency_percentiles: LatencyPercentiles,
    pub active_connections: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LatencyPercentiles {
    pub p50: f64,
    pub p95: f64,
    pub p99: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LatencyBucketCount {
    pub upper_bound: Option<DurationMillis>,
    pub count: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use sql_lens_core::{
        DatabaseType, ProtocolMetadata, ProtocolName, QueryTiming, SqlEventId, SqlEventKind,
        Timestamp,
    };

    fn test_event(
        id: &str,
        status: CaptureStatus,
        duration: DurationMillis,
        connection_id: &str,
    ) -> SqlEvent {
        SqlEvent {
            id: SqlEventId(id.to_owned()),
            timestamp: Timestamp("2026-07-06T09:00:00Z".to_owned()),
            protocol: ProtocolName("mysql".to_owned()),
            database_type: DatabaseType("mysql".to_owned()),
            connection_id: ConnectionId(connection_id.to_owned()),
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

    fn bucket_count(snapshot: &LiveStatisticsSnapshot, upper_bound: Option<u64>) -> u64 {
        snapshot
            .latency_buckets
            .iter()
            .find(|bucket| bucket.upper_bound.map(|duration| duration.0) == upper_bound)
            .expect("latency bucket should exist")
            .count
    }

    #[test]
    fn live_statistics_counts_ok_slow_and_error_events() {
        let mut statistics = LiveStatistics::new();
        let started_at = Instant::now();

        statistics.record_sql_event_at(
            &test_event("evt_1", CaptureStatus::Ok, DurationMillis(1), "conn_1"),
            started_at,
        );
        statistics.record_sql_event_at(
            &test_event("evt_2", CaptureStatus::Slow, DurationMillis(500), "conn_1"),
            started_at + Duration::from_secs(1),
        );
        statistics.record_sql_event_at(
            &test_event("evt_3", CaptureStatus::Error, DurationMillis(10), "conn_1"),
            started_at + Duration::from_secs(2),
        );

        let snapshot = statistics.snapshot_at(started_at + Duration::from_secs(3));

        assert_eq!(snapshot.total_events, 3);
        assert_eq!(snapshot.slow_events, 1);
        assert_eq!(snapshot.error_events, 1);
        assert_eq!(snapshot.qps_window_secs, 60);
        assert!((snapshot.qps - 0.05).abs() < f64::EPSILON);
    }

    #[test]
    fn live_statistics_assigns_latency_buckets() {
        let mut statistics = LiveStatistics::new();
        let started_at = Instant::now();

        statistics.record_sql_event_at(
            &test_event("evt_1", CaptureStatus::Ok, DurationMillis(1), "conn_1"),
            started_at,
        );
        statistics.record_sql_event_at(
            &test_event("evt_2", CaptureStatus::Ok, DurationMillis(50), "conn_1"),
            started_at,
        );
        statistics.record_sql_event_at(
            &test_event("evt_3", CaptureStatus::Ok, DurationMillis(6_000), "conn_1"),
            started_at,
        );

        let snapshot = statistics.snapshot_at(started_at);

        assert_eq!(snapshot.latency_buckets.len(), 9);
        assert_eq!(bucket_count(&snapshot, Some(1)), 1);
        assert_eq!(bucket_count(&snapshot, Some(50)), 1);
        assert_eq!(bucket_count(&snapshot, None), 1);
    }

    #[test]
    fn live_statistics_qps_uses_fixed_recent_window() {
        let mut statistics = LiveStatistics::new();
        let started_at = Instant::now();

        statistics.record_sql_event_at(
            &test_event("evt_1", CaptureStatus::Ok, DurationMillis(1), "conn_1"),
            started_at,
        );
        statistics.record_sql_event_at(
            &test_event("evt_2", CaptureStatus::Ok, DurationMillis(1), "conn_1"),
            started_at + Duration::from_secs(30),
        );

        let snapshot = statistics.snapshot_at(started_at + Duration::from_secs(61));

        assert_eq!(snapshot.total_events, 2);
        assert!((snapshot.qps - (1.0 / 60.0)).abs() < f64::EPSILON);
    }

    #[test]
    fn live_statistics_returns_zero_latency_percentiles_for_empty_state() {
        let mut statistics = LiveStatistics::new();

        let snapshot = statistics.snapshot();

        assert_eq!(
            snapshot.latency_percentiles,
            LatencyPercentiles {
                p50: 0.0,
                p95: 0.0,
                p99: 0.0,
            }
        );
    }

    #[test]
    fn live_statistics_calculates_recent_latency_percentiles() {
        let mut statistics = LiveStatistics::new();
        let started_at = Instant::now();

        for (index, duration_ms) in [10, 20, 30, 40].into_iter().enumerate() {
            statistics.record_sql_event_at(
                &test_event(
                    &format!("evt_{index}"),
                    CaptureStatus::Ok,
                    DurationMillis(duration_ms),
                    "conn_1",
                ),
                started_at + Duration::from_secs(index as u64),
            );
        }

        let snapshot = statistics.snapshot_at(started_at + Duration::from_secs(4));

        assert_eq!(
            snapshot.latency_percentiles,
            LatencyPercentiles {
                p50: 20.0,
                p95: 40.0,
                p99: 40.0,
            }
        );
    }

    #[test]
    fn live_statistics_prunes_latency_samples_outside_recent_window() {
        let mut statistics = LiveStatistics::new();
        let started_at = Instant::now();

        statistics.record_sql_event_at(
            &test_event("evt_1", CaptureStatus::Ok, DurationMillis(1), "conn_1"),
            started_at,
        );
        statistics.record_sql_event_at(
            &test_event("evt_2", CaptureStatus::Ok, DurationMillis(100), "conn_1"),
            started_at + Duration::from_secs(30),
        );

        let snapshot = statistics.snapshot_at(started_at + Duration::from_secs(61));

        assert_eq!(
            snapshot.latency_percentiles,
            LatencyPercentiles {
                p50: 100.0,
                p95: 100.0,
                p99: 100.0,
            }
        );
    }

    #[test]
    fn live_statistics_tracks_active_connections_explicitly() {
        let mut statistics = LiveStatistics::new();
        let conn_1 = ConnectionId("conn_1".to_owned());
        let conn_2 = ConnectionId("conn_2".to_owned());
        let now = Instant::now();

        statistics.record_connection_opened(conn_1.clone());
        statistics.record_connection_opened(conn_1.clone());
        statistics.record_connection_opened(conn_2.clone());

        assert_eq!(statistics.snapshot_at(now).active_connections, 2);

        statistics.record_connection_closed(&conn_1);
        statistics.record_connection_closed(&conn_1);

        assert_eq!(statistics.snapshot_at(now).active_connections, 1);

        statistics.record_connection_closed(&conn_2);

        assert_eq!(statistics.snapshot_at(now).active_connections, 0);
    }
}
