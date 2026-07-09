//! Storage backends for SQL Lens.

mod connection_store;
mod live_statistics;
mod ring_buffer;
mod sqlite_event_store;
mod sqlite_schema;

pub use connection_store::{ConnectionStore, ConnectionUpsertOutcome};
pub use live_statistics::{
    LatencyBucketCount, LatencyPercentiles, LiveStatistics, LiveStatisticsSnapshot,
};
pub use ring_buffer::{
    RingBufferAppendOutcome, RingBufferRetentionOutcome, RingBufferStats, RingBufferStore,
    RingBufferTimelineCursor, RingBufferTimelinePage, RingBufferTimelineQuery, SqlEventFilter,
    SqlEventFilterError,
};
pub use sqlite_event_store::{
    SqliteEventRow, SqliteEventStore, SqliteParameterRow, SqliteRetentionOutcome,
    SqliteTimelineCursor, SqliteTimelinePage, SqliteTimelineQuery, SqliteTimelineQueryError,
};
pub use sqlite_schema::{SQLITE_SCHEMA_VERSION, apply_sqlite_schema};
