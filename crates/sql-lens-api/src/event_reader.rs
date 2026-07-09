use std::{
    num::NonZeroUsize,
    sync::{Arc, Mutex},
};

use sql_lens_core::{SqlEventId, Timestamp};
use sql_lens_storage::{
    RingBufferStore, RingBufferTimelineCursor, RingBufferTimelineQuery, SqlEventFilter,
    SqliteEventStore, SqliteTimelineCursor, SqliteTimelineQuery, SqliteTimelineQueryError,
};
use tokio::sync::RwLock;

use crate::{
    api_error::ApiEndpointError,
    sql_events::{
        SqlEventDetailResponse, SqlEventSummaryResponse, sqlite_detail_response,
        sqlite_summary_response,
    },
};

const RING_CURSOR_PREFIX: &str = "seq_";
const SQLITE_CURSOR_PREFIX: &str = "sqlite_";
const SQLITE_CURSOR_SEPARATOR: char = '|';

#[derive(Debug, Clone)]
pub(crate) enum SqlEventReadStore {
    RingBuffer(Arc<RwLock<RingBufferStore>>),
    Sqlite(Arc<Mutex<SqliteEventStore>>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SqlEventReadQuery {
    pub(crate) limit: NonZeroUsize,
    pub(crate) cursor: Option<String>,
    pub(crate) filter: SqlEventFilter,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SqlEventReadPage {
    pub(crate) items: Vec<SqlEventSummaryResponse>,
    pub(crate) next_cursor: Option<String>,
}

impl SqlEventReadStore {
    pub(crate) fn ring_buffer(event_store: Arc<RwLock<RingBufferStore>>) -> Self {
        Self::RingBuffer(event_store)
    }

    pub(crate) fn sqlite(event_store: SqliteEventStore) -> Self {
        Self::Sqlite(Arc::new(Mutex::new(event_store)))
    }

    pub(crate) async fn query_timeline(
        &self,
        query: SqlEventReadQuery,
    ) -> Result<SqlEventReadPage, ApiEndpointError> {
        match self {
            Self::RingBuffer(store) => {
                let query = RingBufferTimelineQuery {
                    limit: query.limit,
                    cursor: query
                        .cursor
                        .as_deref()
                        .map(decode_ring_cursor)
                        .transpose()?,
                    filter: query.filter,
                };
                let page = {
                    let store = store.read().await;
                    store.query_timeline(query)?
                };

                Ok(SqlEventReadPage {
                    items: page
                        .events
                        .iter()
                        .map(SqlEventSummaryResponse::from)
                        .collect(),
                    next_cursor: page.next_cursor.map(encode_ring_cursor),
                })
            }
            Self::Sqlite(store) => {
                let query = SqliteTimelineQuery {
                    limit: query.limit,
                    cursor: query
                        .cursor
                        .as_deref()
                        .map(decode_sqlite_cursor)
                        .transpose()?,
                    filter: query.filter,
                };
                let page = {
                    let store = lock_sqlite_store(store)?;
                    store.query_timeline(query).map_err(sqlite_timeline_error)?
                };
                let items = page
                    .events
                    .iter()
                    .map(sqlite_summary_response)
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(SqlEventReadPage {
                    items,
                    next_cursor: page.next_cursor.map(encode_sqlite_cursor),
                })
            }
        }
    }

    pub(crate) async fn query_details(
        &self,
        query: SqlEventReadQuery,
    ) -> Result<Vec<SqlEventDetailResponse>, ApiEndpointError> {
        match self {
            Self::RingBuffer(store) => {
                let query = RingBufferTimelineQuery {
                    limit: query.limit,
                    cursor: query
                        .cursor
                        .as_deref()
                        .map(decode_ring_cursor)
                        .transpose()?,
                    filter: query.filter,
                };
                let page = {
                    let store = store.read().await;
                    store.query_timeline(query)?
                };

                Ok(page
                    .events
                    .iter()
                    .map(SqlEventDetailResponse::from)
                    .collect())
            }
            Self::Sqlite(store) => {
                let query = SqliteTimelineQuery {
                    limit: query.limit,
                    cursor: query
                        .cursor
                        .as_deref()
                        .map(decode_sqlite_cursor)
                        .transpose()?,
                    filter: query.filter,
                };
                let details = {
                    let store = lock_sqlite_store(store)?;
                    let page = store.query_timeline(query).map_err(sqlite_timeline_error)?;
                    let mut details = Vec::with_capacity(page.events.len());

                    for row in page.events {
                        let event_id = SqlEventId(row.id.clone());
                        let parameters = store
                            .get_parameter_rows(&event_id)
                            .map_err(sqlite_read_error)?;
                        details.push(sqlite_detail_response(&row, &parameters)?);
                    }

                    details
                };

                Ok(details)
            }
        }
    }

    pub(crate) async fn get_detail(
        &self,
        id: &SqlEventId,
    ) -> Result<Option<SqlEventDetailResponse>, ApiEndpointError> {
        match self {
            Self::RingBuffer(store) => {
                let detail = {
                    let store = store.read().await;
                    store.get(id).map(SqlEventDetailResponse::from)
                };

                Ok(detail)
            }
            Self::Sqlite(store) => {
                let detail = {
                    let store = lock_sqlite_store(store)?;
                    let Some(row) = store.get_event_row(id).map_err(sqlite_read_error)? else {
                        return Ok(None);
                    };
                    let parameters = store.get_parameter_rows(id).map_err(sqlite_read_error)?;

                    sqlite_detail_response(&row, &parameters)?
                };

                Ok(Some(detail))
            }
        }
    }
}

fn lock_sqlite_store(
    store: &Arc<Mutex<SqliteEventStore>>,
) -> Result<std::sync::MutexGuard<'_, SqliteEventStore>, ApiEndpointError> {
    store
        .lock()
        .map_err(|_| ApiEndpointError::storage_unavailable("SQLite event store is unavailable"))
}

fn sqlite_timeline_error(error: SqliteTimelineQueryError) -> ApiEndpointError {
    match error {
        SqliteTimelineQueryError::InvalidFilter(error) => ApiEndpointError::from(error),
        SqliteTimelineQueryError::Sqlite(_) => {
            ApiEndpointError::storage_unavailable("SQLite event read failed")
        }
    }
}

fn sqlite_read_error<E>(_: E) -> ApiEndpointError {
    ApiEndpointError::storage_unavailable("SQLite event read failed")
}

fn encode_ring_cursor(cursor: RingBufferTimelineCursor) -> String {
    format!("{RING_CURSOR_PREFIX}{}", cursor.before_sequence)
}

fn decode_ring_cursor(cursor: &str) -> Result<RingBufferTimelineCursor, ApiEndpointError> {
    let before_sequence = cursor
        .strip_prefix(RING_CURSOR_PREFIX)
        .and_then(|value| value.parse::<u64>().ok())
        .ok_or_else(|| ApiEndpointError::bad_request("invalid cursor", "cursor"))?;

    Ok(RingBufferTimelineCursor { before_sequence })
}

fn encode_sqlite_cursor(cursor: SqliteTimelineCursor) -> String {
    format!(
        "{SQLITE_CURSOR_PREFIX}{}{}{}",
        cursor.before_timestamp.0, SQLITE_CURSOR_SEPARATOR, cursor.before_event_id.0
    )
}

fn decode_sqlite_cursor(cursor: &str) -> Result<SqliteTimelineCursor, ApiEndpointError> {
    let value = cursor
        .strip_prefix(SQLITE_CURSOR_PREFIX)
        .ok_or_else(|| ApiEndpointError::bad_request("invalid cursor", "cursor"))?;
    let (timestamp, event_id) = value
        .split_once(SQLITE_CURSOR_SEPARATOR)
        .filter(|(timestamp, event_id)| !timestamp.is_empty() && !event_id.is_empty())
        .ok_or_else(|| ApiEndpointError::bad_request("invalid cursor", "cursor"))?;

    Ok(SqliteTimelineCursor {
        before_timestamp: Timestamp(timestamp.to_owned()),
        before_event_id: SqlEventId(event_id.to_owned()),
    })
}
