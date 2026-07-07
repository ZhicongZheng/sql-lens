use sql_lens_core::{ConnectionId, ConnectionInfo};
use std::{collections::VecDeque, num::NonZeroUsize};

#[derive(Debug, Clone)]
pub struct ConnectionStore {
    capacity: NonZeroUsize,
    connections: VecDeque<ConnectionInfo>,
}

impl ConnectionStore {
    pub fn new(capacity: NonZeroUsize) -> Self {
        Self {
            capacity,
            connections: VecDeque::with_capacity(capacity.get()),
        }
    }

    pub fn upsert(&mut self, connection: ConnectionInfo) -> ConnectionUpsertOutcome {
        let stored_connection_id = connection.id.clone();
        let existing_position = self
            .connections
            .iter()
            .position(|stored| stored.id == stored_connection_id);
        let replaced_existing = existing_position.is_some();

        if let Some(position) = existing_position {
            self.connections.remove(position);
        }

        let evicted_connection_id =
            if !replaced_existing && self.connections.len() == self.capacity.get() {
                self.connections.pop_front().map(|connection| connection.id)
            } else {
                None
            };

        self.connections.push_back(connection);

        ConnectionUpsertOutcome {
            stored_connection_id,
            replaced_existing,
            evicted_connection_id,
        }
    }

    pub fn list_recent(&self, limit: NonZeroUsize) -> Vec<ConnectionInfo> {
        self.connections
            .iter()
            .rev()
            .take(limit.get())
            .cloned()
            .collect()
    }

    pub fn get(&self, id: &ConnectionId) -> Option<&ConnectionInfo> {
        self.connections
            .iter()
            .find(|connection| &connection.id == id)
    }

    pub fn len(&self) -> usize {
        self.connections.len()
    }

    pub fn capacity(&self) -> usize {
        self.capacity.get()
    }

    pub fn is_empty(&self) -> bool {
        self.connections.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionUpsertOutcome {
    pub stored_connection_id: ConnectionId,
    pub replaced_existing: bool,
    pub evicted_connection_id: Option<ConnectionId>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use sql_lens_core::{ConnectionState, DatabaseType, ProtocolName, Timestamp};

    fn capacity(value: usize) -> NonZeroUsize {
        NonZeroUsize::new(value).expect("test capacity should be non-zero")
    }

    fn test_connection(id: &str, state: ConnectionState) -> ConnectionInfo {
        ConnectionInfo {
            id: ConnectionId(id.to_owned()),
            protocol: ProtocolName("mysql".to_owned()),
            database_type: DatabaseType("mysql".to_owned()),
            client_addr: "127.0.0.1:51000".to_owned(),
            backend_addr: "127.0.0.1:3306".to_owned(),
            user: Some("app".to_owned()),
            database: Some("app".to_owned()),
            state,
            connected_at: Timestamp("2026-07-07T09:00:00Z".to_owned()),
            closed_at: None,
            last_activity_at: Some(Timestamp("2026-07-07T09:00:00Z".to_owned())),
            bytes_in: 0,
            bytes_out: 0,
            query_count: 0,
        }
    }

    fn connection_ids(connections: &[ConnectionInfo]) -> Vec<ConnectionId> {
        connections
            .iter()
            .map(|connection| connection.id.clone())
            .collect()
    }

    #[test]
    fn connection_store_upserts_active_connection() {
        let mut store = ConnectionStore::new(capacity(2));
        let outcome = store.upsert(test_connection("conn_1", ConnectionState::Ready));

        assert_eq!(
            outcome,
            ConnectionUpsertOutcome {
                stored_connection_id: ConnectionId("conn_1".to_owned()),
                replaced_existing: false,
                evicted_connection_id: None,
            }
        );
        assert_eq!(store.len(), 1);
        assert_eq!(
            store
                .get(&ConnectionId("conn_1".to_owned()))
                .expect("connection should exist")
                .state,
            ConnectionState::Ready
        );
    }

    #[test]
    fn connection_store_updates_existing_connection_to_closed() {
        let mut store = ConnectionStore::new(capacity(2));
        store.upsert(test_connection("conn_1", ConnectionState::Ready));
        let mut closed = test_connection("conn_1", ConnectionState::Closed);
        closed.closed_at = Some(Timestamp("2026-07-07T09:01:00Z".to_owned()));
        closed.bytes_in = 10;
        closed.bytes_out = 20;

        let outcome = store.upsert(closed);

        assert!(outcome.replaced_existing);
        assert_eq!(outcome.evicted_connection_id, None);
        assert_eq!(store.len(), 1);
        let stored = store
            .get(&ConnectionId("conn_1".to_owned()))
            .expect("connection should exist");
        assert_eq!(stored.state, ConnectionState::Closed);
        assert_eq!(stored.bytes_in, 10);
        assert_eq!(stored.bytes_out, 20);
    }

    #[test]
    fn connection_store_lists_recent_connections_newest_first() {
        let mut store = ConnectionStore::new(capacity(3));
        store.upsert(test_connection("conn_1", ConnectionState::Ready));
        store.upsert(test_connection("conn_2", ConnectionState::Ready));
        store.upsert(test_connection("conn_3", ConnectionState::Closed));

        let connections = store.list_recent(capacity(2));

        assert_eq!(
            connection_ids(&connections),
            vec![
                ConnectionId("conn_3".to_owned()),
                ConnectionId("conn_2".to_owned())
            ]
        );
    }

    #[test]
    fn connection_store_moves_updated_connection_to_newest_position() {
        let mut store = ConnectionStore::new(capacity(3));
        store.upsert(test_connection("conn_1", ConnectionState::Ready));
        store.upsert(test_connection("conn_2", ConnectionState::Ready));
        store.upsert(test_connection("conn_1", ConnectionState::Closed));

        let connections = store.list_recent(capacity(3));

        assert_eq!(
            connection_ids(&connections),
            vec![
                ConnectionId("conn_1".to_owned()),
                ConnectionId("conn_2".to_owned())
            ]
        );
    }

    #[test]
    fn connection_store_evicts_oldest_updated_connection_when_full() {
        let mut store = ConnectionStore::new(capacity(2));
        store.upsert(test_connection("conn_1", ConnectionState::Ready));
        store.upsert(test_connection("conn_2", ConnectionState::Ready));
        let outcome = store.upsert(test_connection("conn_3", ConnectionState::Ready));

        assert_eq!(
            outcome.evicted_connection_id,
            Some(ConnectionId("conn_1".to_owned()))
        );
        assert!(store.get(&ConnectionId("conn_1".to_owned())).is_none());
        assert!(store.get(&ConnectionId("conn_2".to_owned())).is_some());
        assert!(store.get(&ConnectionId("conn_3".to_owned())).is_some());
    }

    #[test]
    fn connection_store_reports_capacity_and_empty_state() {
        let mut store = ConnectionStore::new(capacity(2));

        assert_eq!(store.capacity(), 2);
        assert!(store.is_empty());

        store.upsert(test_connection("conn_1", ConnectionState::Ready));

        assert!(!store.is_empty());
    }
}
