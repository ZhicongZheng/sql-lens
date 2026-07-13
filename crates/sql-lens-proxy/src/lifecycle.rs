use crate::{BackendDialFailure, BackendDialFailureKind, ForwardingFailure, ForwardingSummary};
use sql_lens_core::{
    ConnectionId, ConnectionInfo, ConnectionState, DatabaseType, ProtocolName, Timestamp,
};
use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

#[derive(Debug, Default)]
pub struct ConnectionLifecycleIdGenerator {
    next_sequence: AtomicU64,
}

impl ConnectionLifecycleIdGenerator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn next_id(&self) -> ConnectionId {
        let sequence = self.next_sequence.fetch_add(1, Ordering::Relaxed) + 1;

        ConnectionId(format!("conn_{sequence}"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionLifecycleRecord {
    info: ConnectionInfo,
    transitions: Vec<ConnectionLifecycleTransition>,
    failure: Option<ConnectionLifecycleFailure>,
}

impl ConnectionLifecycleRecord {
    pub fn accepted(
        id: ConnectionId,
        target_name: Option<String>,
        protocol: ProtocolName,
        database_type: DatabaseType,
        client_addr: impl Into<String>,
        backend_addr: impl Into<String>,
        accepted_at: Timestamp,
    ) -> Self {
        let info = ConnectionInfo {
            id,
            target_name,
            protocol,
            database_type,
            client_addr: client_addr.into(),
            backend_addr: backend_addr.into(),
            user: None,
            database: None,
            state: ConnectionState::Created,
            connected_at: accepted_at.clone(),
            closed_at: None,
            last_activity_at: Some(accepted_at.clone()),
            bytes_in: 0,
            bytes_out: 0,
            query_count: 0,
        };

        Self {
            info,
            transitions: vec![ConnectionLifecycleTransition {
                state: ConnectionState::Created,
                at: accepted_at,
            }],
            failure: None,
        }
    }

    pub fn info(&self) -> &ConnectionInfo {
        &self.info
    }

    pub fn transitions(&self) -> &[ConnectionLifecycleTransition] {
        &self.transitions
    }

    pub fn failure(&self) -> Option<&ConnectionLifecycleFailure> {
        self.failure.as_ref()
    }

    pub fn into_info(self) -> ConnectionInfo {
        self.info
    }

    pub fn mark_backend_connected(&mut self, connected_at: Timestamp) {
        self.transition_to(ConnectionState::BackendConnected, connected_at);
    }

    pub fn mark_forwarding_closed(&mut self, summary: &ForwardingSummary, closed_at: Timestamp) {
        self.info.bytes_in = summary.client_to_backend_bytes;
        self.info.bytes_out = summary.backend_to_client_bytes;
        self.info.closed_at = Some(closed_at.clone());
        self.transition_to(ConnectionState::Closing, closed_at.clone());
        self.transition_to(ConnectionState::Closed, closed_at);
    }

    pub fn mark_backend_dial_failed(&mut self, failure: &BackendDialFailure, failed_at: Timestamp) {
        self.failure = Some(ConnectionLifecycleFailure::from_backend_dial_failure(
            failure,
        ));
        self.info.closed_at = Some(failed_at.clone());
        self.transition_to(ConnectionState::Failed, failed_at);
    }

    pub fn mark_connection_rejected(&mut self, rejected_at: Timestamp) {
        self.failure = Some(ConnectionLifecycleFailure {
            client_addr: self.info.client_addr.clone(),
            backend_addr: self.info.backend_addr.clone(),
            kind: ConnectionLifecycleFailureKind::ConnectionLimit,
        });
        self.info.closed_at = Some(rejected_at.clone());
        self.transition_to(ConnectionState::Failed, rejected_at);
    }

    pub fn mark_forwarding_failed(&mut self, failure: &ForwardingFailure, failed_at: Timestamp) {
        if let Some(bytes_in) = failure.client_to_backend_bytes {
            self.info.bytes_in = bytes_in;
        }

        if let Some(bytes_out) = failure.backend_to_client_bytes {
            self.info.bytes_out = bytes_out;
        }

        self.failure = Some(ConnectionLifecycleFailure::from_forwarding_failure(failure));
        self.info.closed_at = Some(failed_at.clone());
        self.transition_to(ConnectionState::Failed, failed_at);
    }

    fn transition_to(&mut self, state: ConnectionState, at: Timestamp) {
        self.info.state = state;
        self.info.last_activity_at = Some(at.clone());
        self.transitions
            .push(ConnectionLifecycleTransition { state, at });
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionLifecycleTransition {
    pub state: ConnectionState,
    pub at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionLifecycleFailure {
    pub client_addr: String,
    pub backend_addr: String,
    pub kind: ConnectionLifecycleFailureKind,
}

impl ConnectionLifecycleFailure {
    pub fn from_backend_dial_failure(failure: &BackendDialFailure) -> Self {
        let kind = match failure.kind {
            BackendDialFailureKind::Timeout { timeout } => {
                ConnectionLifecycleFailureKind::BackendDialTimeout { timeout }
            }
            BackendDialFailureKind::Connect => ConnectionLifecycleFailureKind::BackendDialConnect,
        };

        Self {
            client_addr: failure.client_peer_addr.to_string(),
            backend_addr: failure.backend_address.clone(),
            kind,
        }
    }

    pub fn from_forwarding_failure(failure: &ForwardingFailure) -> Self {
        Self {
            client_addr: failure.client_peer_addr.to_string(),
            backend_addr: failure.backend_address.clone(),
            kind: ConnectionLifecycleFailureKind::ForwardingIo,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionLifecycleFailureKind {
    ConnectionLimit,
    BackendDialTimeout { timeout: Duration },
    BackendDialConnect,
    ForwardingIo,
}
