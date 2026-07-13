//! Retention enforcement for SQL Lens runtime storage.
//!
//! Periodically enforces configured retention policies on both ring buffer
//! and SQLite storage backends.

use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};

use sql_lens_config::RetentionConfig;
use sql_lens_core::Timestamp;
use sql_lens_storage::{RingBufferStore, SqliteEventStore};
use tokio::sync::RwLock;

/// Retention enforcement service that periodically cleans up old events.
pub struct RetentionEnforcer {
    config: RetentionConfig,
    ring_buffer: Arc<RwLock<RingBufferStore>>,
    sqlite_store: Option<Arc<Mutex<SqliteEventStore>>>,
}

impl RetentionEnforcer {
    /// Create a new retention enforcer.
    pub fn new(
        config: RetentionConfig,
        ring_buffer: Arc<RwLock<RingBufferStore>>,
        sqlite_store: Option<Arc<Mutex<SqliteEventStore>>>,
    ) -> Result<Self, RetentionError> {
        Self::validate_config(&config)?;

        Ok(Self {
            config,
            ring_buffer,
            sqlite_store,
        })
    }

    pub fn validate_config(config: &RetentionConfig) -> Result<(), RetentionError> {
        validate_retention_config(config)
    }

    /// Enforce retention from a blocking worker, never from a Tokio runtime worker.
    pub fn enforce_blocking(&self) -> Result<usize, RetentionError> {
        let mut total_deleted = 0;

        let cutoff = self.parse_max_age_cutoff()?;
        let max_events = configured_max_events(&self.config)?;

        {
            let mut store = self.ring_buffer.blocking_write();
            let before_events = store.len();

            if let Some(cutoff) = &cutoff {
                let outcome = store.delete_events_older_than(cutoff);
                total_deleted += outcome.deleted_event_ids.len();
            }
            if let Some(max_events) = max_events {
                let outcome = store.enforce_max_events(max_events);
                total_deleted += outcome.deleted_event_ids.len();
            }

            tracing::info!(
                before_events,
                after_events = store.len(),
                deleted_events = before_events.saturating_sub(store.len()),
                "ring buffer retention enforcement completed"
            );
        }

        // Enforce retention on SQLite if configured
        if let Some(sqlite) = &self.sqlite_store {
            let mut store = sqlite.lock().map_err(|_| {
                RetentionError::StorageError("retention SQLite lock poisoned".to_owned())
            })?;
            let before_events = store.event_count().map_err(|error| {
                RetentionError::StorageError(format!("SQLite event count failed: {error}"))
            })?;

            if let Some(cutoff) = &cutoff {
                match store.delete_events_older_than(cutoff) {
                    Ok(outcome) => total_deleted += outcome.deleted_event_count,
                    Err(error) => {
                        tracing::error!(error = %error, "SQLite age-based retention failed")
                    }
                }
            }
            if let Some(max_events) = max_events {
                match store.enforce_max_events(max_events) {
                    Ok(outcome) => total_deleted += outcome.deleted_event_count,
                    Err(error) => {
                        tracing::error!(error = %error, "SQLite max events enforcement failed")
                    }
                }
            }

            let after_events = store.event_count().map_err(|error| {
                RetentionError::StorageError(format!("SQLite event count failed: {error}"))
            })?;
            tracing::info!(
                before_events,
                after_events,
                deleted_events = before_events.saturating_sub(after_events),
                "SQLite retention enforcement completed"
            );
        }

        Ok(total_deleted)
    }

    /// Parse max_age configuration into a cutoff timestamp.
    ///
    /// Supports the configured duration formats: `ms`, `s`, `m`, and `h`.
    fn parse_max_age_cutoff(&self) -> Result<Option<Timestamp>, RetentionError> {
        use std::time::{SystemTime, UNIX_EPOCH};

        let age_str = self.config.max_age.trim();
        if age_str.is_empty() || age_str == "0" {
            return Ok(None);
        }

        let duration =
            sql_lens_config::parse_retention_enforcement_interval(age_str).ok_or_else(|| {
                RetentionError::InvalidConfig("retention.max_age is invalid".to_owned())
            })?;
        let now = SystemTime::now();
        let cutoff = now.checked_sub(duration).ok_or_else(|| {
            RetentionError::InvalidConfig(
                "retention.max_age is outside the timestamp range".to_owned(),
            )
        })?;
        let millis = cutoff
            .duration_since(UNIX_EPOCH)
            .map_err(|_| {
                RetentionError::InvalidConfig(
                    "retention.max_age cutoff is before unix epoch".to_owned(),
                )
            })?
            .as_millis();
        Ok(Some(Timestamp(format!("unix_ms:{millis}"))))
    }
}

fn validate_retention_config(config: &RetentionConfig) -> Result<(), RetentionError> {
    if config.max_bytes.is_some() {
        return Err(RetentionError::InvalidConfig(
            "retention.max_bytes is not supported by the runtime".to_owned(),
        ));
    }

    if config.max_age.trim() != ""
        && config.max_age.trim() != "0"
        && sql_lens_config::parse_retention_enforcement_interval(&config.max_age).is_none()
    {
        return Err(RetentionError::InvalidConfig(
            "retention.max_age must be empty, 0, or a positive duration".to_owned(),
        ));
    }

    if config.max_events > usize::MAX as u64 {
        return Err(RetentionError::InvalidConfig(
            "retention.max_events exceeds the platform limit".to_owned(),
        ));
    }

    Ok(())
}

fn configured_max_events(config: &RetentionConfig) -> Result<Option<NonZeroUsize>, RetentionError> {
    if config.max_events == 0 {
        return Ok(None);
    }

    let max_events = usize::try_from(config.max_events).map_err(|_| {
        RetentionError::InvalidConfig("retention.max_events exceeds the platform limit".to_owned())
    })?;
    Ok(NonZeroUsize::new(max_events))
}

/// Error type for retention enforcement operations.
#[allow(dead_code)]
#[derive(Debug)]
pub enum RetentionError {
    InvalidConfig(String),
    StorageError(String),
}

impl std::fmt::Display for RetentionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidConfig(msg) => write!(f, "invalid retention configuration: {}", msg),
            Self::StorageError(msg) => write!(f, "storage operation failed: {}", msg),
        }
    }
}

impl std::error::Error for RetentionError {}
