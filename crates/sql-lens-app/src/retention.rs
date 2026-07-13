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
    ) -> Self {
        Self {
            config,
            ring_buffer,
            sqlite_store,
        }
    }

    /// Enforce retention from a blocking worker, never from a Tokio runtime worker.
    pub fn enforce_blocking(&self) -> Result<usize, RetentionError> {
        let mut total_deleted = 0;

        // Validate that max_bytes is not configured (unsupported)
        if self.config.max_bytes.is_some() {
            return Err(RetentionError::InvalidConfig(
                "max_bytes retention is not supported".to_owned(),
            ));
        }

        // Enforce event count limit on ring buffer
        if self.config.max_events > 0 {
            let max_events =
                NonZeroUsize::new(self.config.max_events as usize).ok_or_else(|| {
                    RetentionError::InvalidConfig("max_events must be greater than zero".to_owned())
                })?;

            let mut store = self.ring_buffer.blocking_write();
            let outcome = store.enforce_max_events(max_events);
            total_deleted += outcome.deleted_event_ids.len();

            if !outcome.deleted_event_ids.is_empty() {
                tracing::info!(
                    deleted_count = outcome.deleted_event_ids.len(),
                    "ring buffer retention enforced max events"
                );
            }
        }

        // Enforce age-based retention on ring buffer
        if let Some(ref cutoff) = self.parse_max_age_cutoff() {
            // Ring buffer age-based retention would require timestamps on entries
            // For now, we log that this needs implementation
            tracing::debug!(
                cutoff = %cutoff.0,
                "age-based retention for ring buffer requires timestamp indexing"
            );
        }

        // Enforce retention on SQLite if configured
        if let Some(sqlite) = &self.sqlite_store {
            // Enforce event count limit
            if self.config.max_events > 0 {
                let max_events =
                    NonZeroUsize::new(self.config.max_events as usize).ok_or_else(|| {
                        RetentionError::InvalidConfig(
                            "max_events must be greater than zero".to_owned(),
                        )
                    })?;

                let mut store = sqlite.lock().map_err(|_| {
                    RetentionError::StorageError("retention SQLite lock poisoned".to_owned())
                })?;
                match store.enforce_max_events(max_events) {
                    Ok(outcome) => {
                        total_deleted += outcome.deleted_event_count;
                        if outcome.deleted_event_count > 0 {
                            tracing::info!(
                                deleted_events = outcome.deleted_event_count,
                                deleted_parameters = outcome.deleted_parameter_count,
                                "SQLite retention enforced max events"
                            );
                        }
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "SQLite max events enforcement failed");
                    }
                }
            }

            // Enforce age-based retention
            if let Some(ref cutoff) = self.parse_max_age_cutoff() {
                let mut store = sqlite.lock().map_err(|_| {
                    RetentionError::StorageError("retention SQLite lock poisoned".to_owned())
                })?;
                match store.delete_events_older_than(cutoff) {
                    Ok(outcome) => {
                        total_deleted += outcome.deleted_event_count;
                        if outcome.deleted_event_count > 0 {
                            tracing::info!(
                                deleted_events = outcome.deleted_event_count,
                                deleted_parameters = outcome.deleted_parameter_count,
                                cutoff = %cutoff.0,
                                "SQLite retention deleted old events"
                            );
                        }
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "SQLite age-based retention failed");
                    }
                }
            }
        }

        Ok(total_deleted)
    }

    /// Parse max_age configuration into a cutoff timestamp.
    ///
    /// Supports formats like "24h", "7d", "1w", etc.
    fn parse_max_age_cutoff(&self) -> Option<Timestamp> {
        use std::time::{SystemTime, UNIX_EPOCH};

        let age_str = &self.config.max_age;
        if age_str.is_empty() || age_str == "0" {
            return None;
        }

        let duration = sql_lens_config::parse_retention_enforcement_interval(age_str)?;
        let now = SystemTime::now();
        let cutoff = now.checked_sub(duration)?;
        let millis = cutoff.duration_since(UNIX_EPOCH).ok()?.as_millis();
        Some(Timestamp(format!("unix_ms:{millis}")))
    }
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
