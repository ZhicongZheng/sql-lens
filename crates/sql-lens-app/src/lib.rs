//! Minimal runtime composition helpers for SQL Lens integration tests.

use std::{
    error::Error,
    fmt, io,
    net::SocketAddr,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use sql_lens_api::{ApiState, HttpServerConfig, HttpServerError, bind_http_server_with_state};
use sql_lens_capture::SlowQueryClassifier;
use sql_lens_core::{ConnectionInfo, DatabaseType, ProtocolName, SqlEvent, Timestamp};
use sql_lens_protocol::{CaptureEventEmitter, ProtocolAdapter, ProtocolConnectionContext};
use sql_lens_protocol_mysql::MysqlProtocolAdapter;
use sql_lens_proxy::{
    AcceptedClient, BackendDialConfig, BackendDialError, BackendDialer,
    ConnectionLifecycleIdGenerator, ConnectionLifecycleRecord, ForwardingError, ForwardingFailure,
    ForwardingSummary, ProxiedConnection, ProxyListenerConfig, ProxyListenerError,
    TcpProxyListener,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::{oneshot, watch},
    task::JoinError,
};

const MYSQL_PROTOCOL_NAME: &str = "mysql";
const FORWARDING_BUFFER_SIZE: usize = 16 * 1024;
const DEFAULT_BACKEND_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug)]
pub struct MinimalMysqlRuntime {
    pub proxy_addr: SocketAddr,
    pub api_addr: SocketAddr,
    api_shutdown_tx: oneshot::Sender<()>,
    proxy_shutdown_tx: watch::Sender<bool>,
    api_task: tokio::task::JoinHandle<Result<(), HttpServerError>>,
    proxy_task: tokio::task::JoinHandle<()>,
}

impl MinimalMysqlRuntime {
    pub async fn shutdown(self) -> Result<(), MinimalMysqlRuntimeError> {
        let _ = self.api_shutdown_tx.send(());
        let _ = self.proxy_shutdown_tx.send(true);

        self.api_task
            .await
            .map_err(MinimalMysqlRuntimeError::Join)??;
        self.proxy_task
            .await
            .map_err(MinimalMysqlRuntimeError::Join)?;

        Ok(())
    }
}

pub async fn start_minimal_mysql_runtime(
    backend_address: impl Into<String>,
) -> Result<MinimalMysqlRuntime, MinimalMysqlRuntimeError> {
    let state = ApiState::default();
    let http_server = bind_http_server_with_state(
        &HttpServerConfig {
            listen: "127.0.0.1:0".to_owned(),
            request_timeout_ms: 30_000,
        },
        state.clone(),
    )
    .await?;
    let api_addr = http_server.local_addr();
    let proxy_listener = TcpProxyListener::bind(ProxyListenerConfig::new("127.0.0.1:0")).await?;
    let proxy_addr = proxy_listener.local_addr()?;
    let backend_config = BackendDialConfig::new(backend_address, DEFAULT_BACKEND_CONNECT_TIMEOUT);

    let (api_shutdown_tx, api_shutdown_rx) = oneshot::channel::<()>();
    let (proxy_shutdown_tx, proxy_shutdown_rx) = watch::channel(false);

    let api_task = tokio::spawn(http_server.serve_with_shutdown(async move {
        let _ = api_shutdown_rx.await;
    }));
    let proxy_task = tokio::spawn(run_mysql_proxy(
        proxy_listener,
        backend_config,
        state,
        proxy_shutdown_rx,
    ));

    Ok(MinimalMysqlRuntime {
        proxy_addr,
        api_addr,
        api_shutdown_tx,
        proxy_shutdown_tx,
        api_task,
        proxy_task,
    })
}

async fn run_mysql_proxy(
    listener: TcpProxyListener,
    backend_config: BackendDialConfig,
    state: ApiState,
    mut shutdown: watch::Receiver<bool>,
) {
    let id_generator = ConnectionLifecycleIdGenerator::new();

    loop {
        if *shutdown.borrow_and_update() {
            return;
        }

        tokio::select! {
            biased;

            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow_and_update() {
                    return;
                }
            }
            accepted = listener.accept() => {
                match accepted {
                    Ok(accepted) => {
                        handle_accepted_mysql_client(
                            accepted,
                            backend_config.clone(),
                            state.clone(),
                            id_generator.next_id(),
                        )
                        .await;
                    }
                    Err(source) => {
                        tracing::warn!(error = %source, "failed to accept MySQL proxy client");
                    }
                }
            }
        }
    }
}

async fn handle_accepted_mysql_client(
    accepted: AcceptedClient,
    backend_config: BackendDialConfig,
    state: ApiState,
    connection_id: sql_lens_core::ConnectionId,
) {
    let client_peer_addr = accepted.peer_addr();

    match BackendDialer::dial(accepted, &backend_config).await {
        Ok(connection) => {
            let connection_info = runtime_connection_info(
                connection_id,
                client_peer_addr,
                connection.backend_address().to_owned(),
            );
            tokio::spawn(async move {
                if let Err(source) =
                    forward_mysql_connection(connection, connection_info, state).await
                {
                    tracing::warn!(error = %source, "MySQL proxy forwarding failed");
                }
            });
        }
        Err(source) => {
            tracing::warn!(error = %source, "failed to dial MySQL backend");
        }
    }
}

fn runtime_connection_info(
    connection_id: sql_lens_core::ConnectionId,
    client_addr: SocketAddr,
    backend_addr: String,
) -> ConnectionInfo {
    let record = ConnectionLifecycleRecord::accepted(
        connection_id,
        ProtocolName(MYSQL_PROTOCOL_NAME.to_owned()),
        DatabaseType(MYSQL_PROTOCOL_NAME.to_owned()),
        client_addr.to_string(),
        backend_addr,
        runtime_timestamp(),
    );

    record.into_info()
}

async fn forward_mysql_connection(
    connection: ProxiedConnection,
    connection_info: ConnectionInfo,
    state: ApiState,
) -> Result<ForwardingSummary, ForwardingError> {
    let (mut client_stream, mut backend_stream, client_peer_addr, backend_address) =
        connection.into_parts();
    let adapter = MysqlProtocolAdapter::new();
    let context = ProtocolConnectionContext::new(connection_info);
    let mut protocol_state = adapter.create_connection_state(&context);
    let mut client_to_backend_bytes = 0_u64;
    let mut backend_to_client_bytes = 0_u64;
    let mut client_open = true;
    let mut backend_open = true;
    let mut client_buffer = [0_u8; FORWARDING_BUFFER_SIZE];
    let mut backend_buffer = [0_u8; FORWARDING_BUFFER_SIZE];

    while client_open || backend_open {
        tokio::select! {
            client_read = client_stream.read(&mut client_buffer), if client_open => {
                let bytes_read = client_read.map_err(|source| forwarding_io_error(
                    client_peer_addr,
                    backend_address.clone(),
                    client_to_backend_bytes,
                    backend_to_client_bytes,
                    source,
                ))?;

                if bytes_read == 0 {
                    client_open = false;
                    backend_stream.shutdown().await.map_err(|source| forwarding_io_error(
                        client_peer_addr,
                        backend_address.clone(),
                        client_to_backend_bytes,
                        backend_to_client_bytes,
                        source,
                    ))?;
                    continue;
                }

                observe_client_bytes(
                    &adapter,
                    protocol_state.as_mut(),
                    &client_buffer[..bytes_read],
                );
                backend_stream.write_all(&client_buffer[..bytes_read]).await.map_err(|source| {
                    forwarding_io_error(
                        client_peer_addr,
                        backend_address.clone(),
                        client_to_backend_bytes,
                        backend_to_client_bytes,
                        source,
                    )
                })?;
                client_to_backend_bytes += bytes_read as u64;
            }
            backend_read = backend_stream.read(&mut backend_buffer), if backend_open => {
                let bytes_read = backend_read.map_err(|source| forwarding_io_error(
                    client_peer_addr,
                    backend_address.clone(),
                    client_to_backend_bytes,
                    backend_to_client_bytes,
                    source,
                ))?;

                if bytes_read == 0 {
                    backend_open = false;
                    client_stream.shutdown().await.map_err(|source| forwarding_io_error(
                        client_peer_addr,
                        backend_address.clone(),
                        client_to_backend_bytes,
                        backend_to_client_bytes,
                        source,
                    ))?;
                    continue;
                }

                let events = observe_backend_bytes(
                    &adapter,
                    protocol_state.as_mut(),
                    &backend_buffer[..bytes_read],
                );
                client_stream.write_all(&backend_buffer[..bytes_read]).await.map_err(|source| {
                    forwarding_io_error(
                        client_peer_addr,
                        backend_address.clone(),
                        client_to_backend_bytes,
                        backend_to_client_bytes,
                        source,
                    )
                })?;
                backend_to_client_bytes += bytes_read as u64;
                store_sql_events(&state, events).await;
            }
        }
    }

    Ok(ForwardingSummary {
        client_peer_addr,
        backend_address,
        client_to_backend_bytes,
        backend_to_client_bytes,
    })
}

fn observe_client_bytes(
    adapter: &MysqlProtocolAdapter,
    protocol_state: &mut dyn sql_lens_protocol::ProtocolConnectionState,
    bytes: &[u8],
) {
    let mut events = VecCaptureEventEmitter::default();
    if let Err(source) = adapter.observe_client_bytes(protocol_state, bytes, &mut events) {
        tracing::warn!(error = %source, "failed to observe MySQL client bytes");
    }
}

fn observe_backend_bytes(
    adapter: &MysqlProtocolAdapter,
    protocol_state: &mut dyn sql_lens_protocol::ProtocolConnectionState,
    bytes: &[u8],
) -> Vec<SqlEvent> {
    let mut events = VecCaptureEventEmitter::default();
    if let Err(source) = adapter.observe_backend_bytes(protocol_state, bytes, &mut events) {
        tracing::warn!(error = %source, "failed to observe MySQL backend bytes");
    }

    events.events
}

async fn store_sql_events(state: &ApiState, events: Vec<SqlEvent>) {
    if events.is_empty() {
        return;
    }

    let classifier = SlowQueryClassifier::default();
    let broadcaster = state.sql_event_broadcaster();
    let event_store = state.event_store();
    let live_statistics = state.live_statistics();
    let mut store = event_store.write().await;
    let mut statistics = live_statistics.write().await;

    for event in events {
        let event = classifier.classify(event);
        let _ = broadcaster.publish(event.clone());
        statistics.record_sql_event(&event);
        store.append(event);
    }
}

fn forwarding_io_error(
    client_peer_addr: SocketAddr,
    backend_address: String,
    client_to_backend_bytes: u64,
    backend_to_client_bytes: u64,
    source: io::Error,
) -> ForwardingError {
    ForwardingError::Io {
        failure: ForwardingFailure {
            client_peer_addr,
            backend_address,
            client_to_backend_bytes: Some(client_to_backend_bytes),
            backend_to_client_bytes: Some(backend_to_client_bytes),
        },
        source,
    }
}

fn runtime_timestamp() -> Timestamp {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();

    Timestamp(format!("unix_ms:{millis}"))
}

#[derive(Debug, Default)]
struct VecCaptureEventEmitter {
    events: Vec<SqlEvent>,
}

impl CaptureEventEmitter for VecCaptureEventEmitter {
    fn emit(&mut self, event: SqlEvent) {
        self.events.push(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sql_lens_core::{
        CaptureStatus, ConnectionId, DurationMillis, ProtocolMetadata, QueryTiming, SqlEventId,
        SqlEventKind,
    };

    #[tokio::test]
    async fn store_sql_events_classifies_slow_events_before_storage_and_statistics() {
        let state = ApiState::default();
        let event_id = SqlEventId("evt_slow".to_owned());

        store_sql_events(
            &state,
            vec![test_event(
                event_id.clone(),
                CaptureStatus::Ok,
                DurationMillis(sql_lens_capture::DEFAULT_SLOW_THRESHOLD_MS),
            )],
        )
        .await;

        let event_store = state.event_store();
        let store = event_store.read().await;
        let stored = store
            .get(&event_id)
            .expect("classified event should be stored");
        assert_eq!(stored.status, CaptureStatus::Slow);
        drop(store);

        let live_statistics = state.live_statistics();
        let mut statistics = live_statistics.write().await;
        let snapshot = statistics.snapshot();
        assert_eq!(snapshot.total_events, 1);
        assert_eq!(snapshot.slow_events, 1);
        assert_eq!(snapshot.error_events, 0);
    }

    #[tokio::test]
    async fn store_sql_events_keeps_below_threshold_events_ok() {
        let state = ApiState::default();
        let event_id = SqlEventId("evt_ok".to_owned());

        store_sql_events(
            &state,
            vec![test_event(
                event_id.clone(),
                CaptureStatus::Ok,
                DurationMillis(sql_lens_capture::DEFAULT_SLOW_THRESHOLD_MS - 1),
            )],
        )
        .await;

        let event_store = state.event_store();
        let store = event_store.read().await;
        let stored = store
            .get(&event_id)
            .expect("classified event should be stored");
        assert_eq!(stored.status, CaptureStatus::Ok);
    }

    fn test_event(id: SqlEventId, status: CaptureStatus, duration: DurationMillis) -> SqlEvent {
        SqlEvent {
            id,
            timestamp: Timestamp("2026-07-06T09:00:00Z".to_owned()),
            protocol: ProtocolName(MYSQL_PROTOCOL_NAME.to_owned()),
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
                protocol: ProtocolName(MYSQL_PROTOCOL_NAME.to_owned()),
                fields: Vec::new(),
            },
        }
    }
}

#[derive(Debug)]
pub enum MinimalMysqlRuntimeError {
    Http(HttpServerError),
    ProxyListener(ProxyListenerError),
    BackendDial(BackendDialError),
    Join(JoinError),
}

impl fmt::Display for MinimalMysqlRuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Http(source) => write!(f, "minimal MySQL runtime HTTP server failed: {source}"),
            Self::ProxyListener(source) => {
                write!(f, "minimal MySQL runtime proxy listener failed: {source}")
            }
            Self::BackendDial(source) => {
                write!(f, "minimal MySQL runtime backend dial failed: {source}")
            }
            Self::Join(source) => write!(f, "minimal MySQL runtime task failed: {source}"),
        }
    }
}

impl Error for MinimalMysqlRuntimeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Http(source) => Some(source),
            Self::ProxyListener(source) => Some(source),
            Self::BackendDial(source) => Some(source),
            Self::Join(source) => Some(source),
        }
    }
}

impl From<HttpServerError> for MinimalMysqlRuntimeError {
    fn from(source: HttpServerError) -> Self {
        Self::Http(source)
    }
}

impl From<ProxyListenerError> for MinimalMysqlRuntimeError {
    fn from(source: ProxyListenerError) -> Self {
        Self::ProxyListener(source)
    }
}

impl From<BackendDialError> for MinimalMysqlRuntimeError {
    fn from(source: BackendDialError) -> Self {
        Self::BackendDial(source)
    }
}
