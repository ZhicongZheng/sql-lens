//! Minimal runtime composition helpers for SQL Lens integration tests.

use std::{
    error::Error,
    fmt, io,
    net::SocketAddr,
    num::NonZeroUsize,
    path::PathBuf,
    sync::mpsc::{SyncSender, TrySendError, sync_channel},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use sql_lens_api::{ApiState, HttpServerConfig, HttpServerError, bind_http_server_with_state};
use sql_lens_capture::SlowQueryClassifier;
use sql_lens_config::{
    DatabaseType as ConfigDatabaseType, ProxyTargetConfig, SqlLensConfig, StorageConfig,
    StorageType,
};
use sql_lens_core::{ConnectionInfo, DatabaseType, ProtocolName, SqlEvent, Timestamp};
use sql_lens_protocol::{CaptureEventEmitter, ProtocolAdapter, ProtocolConnectionContext};
use sql_lens_protocol_mysql::MysqlProtocolAdapter;
use sql_lens_proxy::{
    AcceptedClient, BackendDialConfig, BackendDialError, BackendDialer,
    ConnectionLifecycleIdGenerator, ConnectionLifecycleRecord, ForwardingError, ForwardingFailure,
    ForwardingSummary, ProxiedConnection, ProxyListenerConfig, ProxyListenerError,
    TcpProxyListener,
};
use sql_lens_storage::{RingBufferStore, SqliteEventStore};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::{oneshot, watch},
    task::JoinError,
};

const MYSQL_PROTOCOL_NAME: &str = "mysql";
const FORWARDING_BUFFER_SIZE: usize = 16 * 1024;
const DEFAULT_BACKEND_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const SQLITE_PERSISTENCE_CHANNEL_CAPACITY: usize = 1024;

#[derive(Debug)]
pub struct MinimalMysqlRuntime {
    pub proxy_addr: SocketAddr,
    pub proxy_targets: Vec<MinimalMysqlRuntimeTarget>,
    pub api_addr: SocketAddr,
    api_shutdown_tx: oneshot::Sender<()>,
    proxy_shutdown_tx: watch::Sender<bool>,
    api_task: tokio::task::JoinHandle<Result<(), HttpServerError>>,
    proxy_tasks: Vec<tokio::task::JoinHandle<()>>,
    persistence: EventPersistence,
    sqlite_worker: Option<thread::JoinHandle<()>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MinimalMysqlRuntimeTarget {
    pub name: String,
    pub proxy_addr: SocketAddr,
    pub database_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MinimalMysqlTargetConfig {
    pub name: String,
    pub listen: String,
    pub backend_address: String,
    pub database_type: String,
}

impl MinimalMysqlTargetConfig {
    pub fn new(
        name: impl Into<String>,
        listen: impl Into<String>,
        backend_address: impl Into<String>,
        database_type: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            listen: listen.into(),
            backend_address: backend_address.into(),
            database_type: database_type.into(),
        }
    }
}

impl From<&ProxyTargetConfig> for MinimalMysqlTargetConfig {
    fn from(target: &ProxyTargetConfig) -> Self {
        Self {
            name: target.name.clone(),
            listen: target.listen.clone(),
            backend_address: target.backend_address.clone(),
            database_type: config_database_type_value(target.database_type).to_owned(),
        }
    }
}

#[derive(Debug, Clone)]
struct MysqlProxyTargetRuntimeConfig {
    name: String,
    database_type: DatabaseType,
    backend_config: BackendDialConfig,
}

impl MinimalMysqlRuntime {
    pub async fn shutdown(self) -> Result<(), MinimalMysqlRuntimeError> {
        let Self {
            api_shutdown_tx,
            proxy_shutdown_tx,
            api_task,
            proxy_tasks,
            persistence,
            sqlite_worker,
            ..
        } = self;

        let _ = api_shutdown_tx.send(());
        let _ = proxy_shutdown_tx.send(true);

        api_task.await.map_err(MinimalMysqlRuntimeError::Join)??;
        for proxy_task in proxy_tasks {
            proxy_task.await.map_err(MinimalMysqlRuntimeError::Join)?;
        }
        drop(persistence);

        if let Some(worker) = sqlite_worker {
            worker
                .join()
                .map_err(|_| MinimalMysqlRuntimeError::SqlitePersistenceWorkerPanicked)?;
        }

        Ok(())
    }
}

pub async fn start_minimal_mysql_runtime(
    backend_address: impl Into<String>,
) -> Result<MinimalMysqlRuntime, MinimalMysqlRuntimeError> {
    start_minimal_mysql_runtime_with_targets(vec![MinimalMysqlTargetConfig::new(
        "default",
        "127.0.0.1:0",
        backend_address,
        MYSQL_PROTOCOL_NAME,
    )])
    .await
}

pub async fn start_runtime_from_config(
    config: &SqlLensConfig,
) -> Result<MinimalMysqlRuntime, MinimalMysqlRuntimeError> {
    let targets = config
        .effective_targets()
        .iter()
        .map(MinimalMysqlTargetConfig::from)
        .collect();
    let backend_connect_timeout = Duration::from_millis(config.proxy.connect_timeout_ms);
    let runtime_storage = RuntimeStorage::from_config(&config.storage)?;

    start_minimal_mysql_runtime_with_runtime_storage(
        HttpServerConfig::from(&config.web),
        backend_connect_timeout,
        targets,
        runtime_storage,
    )
    .await
}

pub async fn start_minimal_mysql_runtime_with_targets(
    targets: Vec<MinimalMysqlTargetConfig>,
) -> Result<MinimalMysqlRuntime, MinimalMysqlRuntimeError> {
    start_minimal_mysql_runtime_with_options(
        HttpServerConfig {
            listen: "127.0.0.1:0".to_owned(),
            cors_origins: Vec::new(),
            request_timeout_ms: 30_000,
        },
        DEFAULT_BACKEND_CONNECT_TIMEOUT,
        targets,
    )
    .await
}

pub async fn start_minimal_mysql_runtime_with_options(
    http_config: HttpServerConfig,
    backend_connect_timeout: Duration,
    targets: Vec<MinimalMysqlTargetConfig>,
) -> Result<MinimalMysqlRuntime, MinimalMysqlRuntimeError> {
    start_minimal_mysql_runtime_with_runtime_storage(
        http_config,
        backend_connect_timeout,
        targets,
        RuntimeStorage::ring_buffer_default(),
    )
    .await
}

async fn start_minimal_mysql_runtime_with_runtime_storage(
    http_config: HttpServerConfig,
    backend_connect_timeout: Duration,
    targets: Vec<MinimalMysqlTargetConfig>,
    runtime_storage: RuntimeStorage,
) -> Result<MinimalMysqlRuntime, MinimalMysqlRuntimeError> {
    if targets.is_empty() {
        return Err(MinimalMysqlRuntimeError::NoProxyTargets);
    }

    let RuntimeStorage {
        event_store,
        persistence,
        sqlite_worker,
    } = runtime_storage;
    let state = ApiState::new(event_store);
    let http_server = bind_http_server_with_state(&http_config, state.clone()).await?;
    let api_addr = http_server.local_addr();
    tracing::info!(%api_addr, "SQL Lens API server listening");
    let mut bound_targets = Vec::with_capacity(targets.len());
    let mut proxy_targets = Vec::with_capacity(targets.len());

    for target in targets {
        let proxy_listener =
            TcpProxyListener::bind(ProxyListenerConfig::new(target.listen.clone())).await?;
        let proxy_addr = proxy_listener.local_addr()?;
        tracing::info!(
            target_name = %target.name,
            database_type = %target.database_type,
            %proxy_addr,
            "SQL Lens proxy target listening",
        );
        let runtime_config = MysqlProxyTargetRuntimeConfig {
            name: target.name.clone(),
            database_type: DatabaseType(target.database_type.clone()),
            backend_config: BackendDialConfig::new(target.backend_address, backend_connect_timeout),
        };

        proxy_targets.push(MinimalMysqlRuntimeTarget {
            name: target.name,
            proxy_addr,
            database_type: target.database_type,
        });
        bound_targets.push((proxy_listener, runtime_config));
    }

    let proxy_addr = proxy_targets[0].proxy_addr;

    let (api_shutdown_tx, api_shutdown_rx) = oneshot::channel::<()>();
    let (proxy_shutdown_tx, proxy_shutdown_rx) = watch::channel(false);

    let api_task = tokio::spawn(http_server.serve_with_shutdown(async move {
        let _ = api_shutdown_rx.await;
    }));
    let proxy_tasks = bound_targets
        .into_iter()
        .map(|(proxy_listener, runtime_config)| {
            tokio::spawn(run_mysql_proxy(
                proxy_listener,
                runtime_config,
                state.clone(),
                persistence.clone(),
                proxy_shutdown_rx.clone(),
            ))
        })
        .collect();

    Ok(MinimalMysqlRuntime {
        proxy_addr,
        proxy_targets,
        api_addr,
        api_shutdown_tx,
        proxy_shutdown_tx,
        api_task,
        proxy_tasks,
        persistence,
        sqlite_worker,
    })
}

#[derive(Debug)]
struct RuntimeStorage {
    event_store: RingBufferStore,
    persistence: EventPersistence,
    sqlite_worker: Option<thread::JoinHandle<()>>,
}

impl RuntimeStorage {
    fn from_config(config: &StorageConfig) -> Result<Self, MinimalMysqlRuntimeError> {
        let event_store = RingBufferStore::new(storage_capacity(config.capacity)?);

        match config.storage_type {
            StorageType::RingBuffer => Ok(Self {
                event_store,
                persistence: EventPersistence::default(),
                sqlite_worker: None,
            }),
            StorageType::Sqlite => {
                let path = sqlite_storage_path(config)?;
                let store = SqliteEventStore::open(&path).map_err(|source| {
                    MinimalMysqlRuntimeError::SqliteStorage {
                        path: path.clone(),
                        source: Box::new(source),
                    }
                })?;
                let (persistence, sqlite_worker) = EventPersistence::sqlite(store);
                tracing::info!(path = %path.display(), "SQL Lens SQLite persistence enabled");

                Ok(Self {
                    event_store,
                    persistence,
                    sqlite_worker: Some(sqlite_worker),
                })
            }
            StorageType::DuckDb => Err(MinimalMysqlRuntimeError::StorageConfig(
                "storage.type = \"duckdb\" is not supported by app runtime yet".to_owned(),
            )),
        }
    }

    fn ring_buffer_default() -> Self {
        let capacity = NonZeroUsize::new(sql_lens_api::DEFAULT_EVENT_STORE_CAPACITY)
            .expect("default event store capacity should be non-zero");

        Self {
            event_store: RingBufferStore::new(capacity),
            persistence: EventPersistence::default(),
            sqlite_worker: None,
        }
    }
}

fn storage_capacity(capacity: u64) -> Result<NonZeroUsize, MinimalMysqlRuntimeError> {
    let capacity = usize::try_from(capacity).unwrap_or(usize::MAX);

    NonZeroUsize::new(capacity).ok_or_else(|| {
        MinimalMysqlRuntimeError::StorageConfig(
            "storage.capacity must be greater than zero".to_owned(),
        )
    })
}

fn sqlite_storage_path(config: &StorageConfig) -> Result<PathBuf, MinimalMysqlRuntimeError> {
    let path = config.path.trim();
    if path.is_empty() {
        return Err(MinimalMysqlRuntimeError::StorageConfig(
            "storage.path is required when storage.type = \"sqlite\"".to_owned(),
        ));
    }

    Ok(PathBuf::from(path))
}

#[derive(Debug, Clone, Default)]
struct EventPersistence {
    sqlite_tx: Option<SyncSender<SqlEvent>>,
}

impl EventPersistence {
    fn sqlite(mut store: SqliteEventStore) -> (Self, thread::JoinHandle<()>) {
        let (sqlite_tx, sqlite_rx) = sync_channel::<SqlEvent>(SQLITE_PERSISTENCE_CHANNEL_CAPACITY);
        let worker = thread::spawn(move || {
            while let Ok(event) = sqlite_rx.recv() {
                let event_id = event.id.clone();
                if let Err(source) = store.insert_event(&event) {
                    tracing::warn!(
                        event_id = %event_id.0,
                        error = %source,
                        "failed to persist SQL event to SQLite",
                    );
                }
            }
        });

        (
            Self {
                sqlite_tx: Some(sqlite_tx),
            },
            worker,
        )
    }

    fn persist(&self, event: SqlEvent) {
        let Some(sqlite_tx) = &self.sqlite_tx else {
            return;
        };

        let event_id = event.id.clone();
        match sqlite_tx.try_send(event) {
            Ok(()) => {}
            Err(TrySendError::Full(_)) => {
                tracing::warn!(
                    event_id = %event_id.0,
                    "SQLite persistence queue is full; dropping persisted event copy",
                );
            }
            Err(TrySendError::Disconnected(_)) => {
                tracing::warn!(
                    event_id = %event_id.0,
                    "SQLite persistence worker is stopped; dropping persisted event copy",
                );
            }
        }
    }
}

async fn run_mysql_proxy(
    listener: TcpProxyListener,
    target_config: MysqlProxyTargetRuntimeConfig,
    state: ApiState,
    persistence: EventPersistence,
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
                            target_config.clone(),
                            state.clone(),
                            persistence.clone(),
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
    target_config: MysqlProxyTargetRuntimeConfig,
    state: ApiState,
    persistence: EventPersistence,
    connection_id: sql_lens_core::ConnectionId,
) {
    let client_peer_addr = accepted.peer_addr();

    match BackendDialer::dial(accepted, &target_config.backend_config).await {
        Ok(connection) => {
            let connection_info = runtime_connection_info(
                connection_id,
                target_config.name,
                target_config.database_type,
                client_peer_addr,
                connection.backend_address().to_owned(),
            );
            tokio::spawn(async move {
                if let Err(source) =
                    forward_mysql_connection(connection, connection_info, state, persistence).await
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
    target_name: String,
    database_type: DatabaseType,
    client_addr: SocketAddr,
    backend_addr: String,
) -> ConnectionInfo {
    let record = ConnectionLifecycleRecord::accepted(
        connection_id,
        Some(target_name),
        ProtocolName(MYSQL_PROTOCOL_NAME.to_owned()),
        database_type,
        client_addr.to_string(),
        backend_addr,
        runtime_timestamp(),
    );

    record.into_info()
}

fn config_database_type_value(database_type: ConfigDatabaseType) -> &'static str {
    database_type.config_value()
}

async fn forward_mysql_connection(
    connection: ProxiedConnection,
    connection_info: ConnectionInfo,
    state: ApiState,
    persistence: EventPersistence,
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
                store_sql_events(&state, &persistence, events).await;
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

async fn store_sql_events(state: &ApiState, persistence: &EventPersistence, events: Vec<SqlEvent>) {
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
        store.append(event.clone());
        persistence.persist(event);
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

    #[test]
    fn runtime_connection_info_carries_target_identity() {
        let info = runtime_connection_info(
            ConnectionId("conn_1".to_owned()),
            "starrocks-local".to_owned(),
            DatabaseType("starrocks".to_owned()),
            "127.0.0.1:51000".parse().expect("valid client address"),
            "127.0.0.1:9030".to_owned(),
        );

        assert_eq!(info.target_name.as_deref(), Some("starrocks-local"));
        assert_eq!(info.database_type, DatabaseType("starrocks".to_owned()));
        assert_eq!(info.protocol, ProtocolName(MYSQL_PROTOCOL_NAME.to_owned()));
        assert_eq!(info.backend_addr, "127.0.0.1:9030");
    }

    #[test]
    fn minimal_target_config_uses_configured_proxy_target_values() {
        let target = ProxyTargetConfig {
            name: "starrocks-local".to_owned(),
            listen: "127.0.0.1:9037".to_owned(),
            protocol: sql_lens_config::Protocol::MySql,
            database_type: ConfigDatabaseType::StarRocks,
            backend_address: "127.0.0.1:9030".to_owned(),
        };

        assert_eq!(
            MinimalMysqlTargetConfig::from(&target),
            MinimalMysqlTargetConfig {
                name: "starrocks-local".to_owned(),
                listen: "127.0.0.1:9037".to_owned(),
                backend_address: "127.0.0.1:9030".to_owned(),
                database_type: "starrocks".to_owned(),
            }
        );
    }

    #[tokio::test]
    async fn minimal_runtime_binds_multiple_proxy_targets() {
        let runtime = start_minimal_mysql_runtime_with_targets(vec![
            MinimalMysqlTargetConfig::new("mysql-local", "127.0.0.1:0", "127.0.0.1:3306", "mysql"),
            MinimalMysqlTargetConfig::new(
                "starrocks-local",
                "127.0.0.1:0",
                "127.0.0.1:9030",
                "starrocks",
            ),
        ])
        .await
        .expect("multi-target runtime should bind ephemeral listeners");

        assert_eq!(runtime.proxy_targets.len(), 2);
        assert_eq!(runtime.proxy_targets[0].name, "mysql-local");
        assert_eq!(runtime.proxy_targets[0].database_type, "mysql");
        assert_eq!(runtime.proxy_targets[1].name, "starrocks-local");
        assert_eq!(runtime.proxy_targets[1].database_type, "starrocks");
        assert_ne!(
            runtime.proxy_targets[0].proxy_addr,
            runtime.proxy_targets[1].proxy_addr
        );
        assert_eq!(runtime.proxy_addr, runtime.proxy_targets[0].proxy_addr);

        runtime
            .shutdown()
            .await
            .expect("runtime should shut down all proxy tasks");
    }

    #[tokio::test]
    async fn minimal_runtime_rejects_empty_proxy_targets() {
        let error = start_minimal_mysql_runtime_with_targets(Vec::new())
            .await
            .expect_err("empty target list should fail");

        assert!(matches!(error, MinimalMysqlRuntimeError::NoProxyTargets));
    }

    #[tokio::test]
    async fn runtime_from_config_binds_configured_web_and_effective_targets() {
        let mysql_listen = unused_loopback_addr();
        let starrocks_listen = unused_loopback_addr();
        let config = SqlLensConfig::from_toml_str(&format!(
            r#"
[[targets]]
name = "mysql-local"
listen = "{mysql_listen}"
protocol = "mysql"
database_type = "mysql"
backend_address = "127.0.0.1:3306"

[[targets]]
name = "starrocks-local"
listen = "{starrocks_listen}"
protocol = "mysql"
database_type = "starrocks"
backend_address = "127.0.0.1:9030"

[web]
listen = "127.0.0.1:0"
request_timeout_ms = 12345

[proxy]
connect_timeout_ms = 250
"#,
        ))
        .expect("config should parse");
        config.validate().expect("config should validate");

        let runtime = start_runtime_from_config(&config)
            .await
            .expect("runtime should bind configured ephemeral listeners");

        assert_eq!(runtime.api_addr.ip().to_string(), "127.0.0.1");
        assert_ne!(runtime.api_addr.port(), 0);
        assert_eq!(runtime.proxy_targets.len(), 2);
        assert_eq!(runtime.proxy_targets[0].name, "mysql-local");
        assert_eq!(runtime.proxy_targets[1].name, "starrocks-local");
        assert_ne!(
            runtime.proxy_targets[0].proxy_addr,
            runtime.proxy_targets[1].proxy_addr
        );

        runtime
            .shutdown()
            .await
            .expect("runtime should shut down cleanly");
    }

    #[tokio::test]
    async fn runtime_from_config_rejects_sqlite_without_path() {
        let listen = unused_loopback_addr();
        let config = SqlLensConfig::from_toml_str(&format!(
            r#"
[proxy]
listen = "{listen}"

[web]
listen = "127.0.0.1:0"

[storage]
type = "sqlite"
path = "   "
"#,
        ))
        .expect("config should parse");
        config.validate().expect("config should validate");

        let error = start_runtime_from_config(&config)
            .await
            .expect_err("sqlite storage without a path should fail startup");

        assert!(matches!(
            error,
            MinimalMysqlRuntimeError::StorageConfig(message)
                if message.contains("storage.path is required")
        ));
    }

    #[tokio::test]
    async fn runtime_from_config_opens_configured_sqlite_storage() {
        let listen = unused_loopback_addr();
        let path = temporary_sqlite_path("runtime-open");
        let config = SqlLensConfig::from_toml_str(&format!(
            r#"
[proxy]
listen = "{listen}"

[web]
listen = "127.0.0.1:0"

[storage]
type = "sqlite"
path = "{}"
"#,
            path.display()
        ))
        .expect("config should parse");
        config.validate().expect("config should validate");

        let runtime = start_runtime_from_config(&config)
            .await
            .expect("sqlite runtime should start");
        runtime
            .shutdown()
            .await
            .expect("runtime should shut down cleanly");

        assert!(path.exists(), "sqlite file should be created");
        let _ = std::fs::remove_file(path);
    }

    fn unused_loopback_addr() -> SocketAddr {
        let listener =
            std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral test port");
        listener.local_addr().expect("read ephemeral test port")
    }

    #[tokio::test]
    async fn store_sql_events_classifies_slow_events_before_storage_and_statistics() {
        let state = ApiState::default();
        let event_id = SqlEventId("evt_slow".to_owned());

        store_sql_events(
            &state,
            &EventPersistence::default(),
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
            &EventPersistence::default(),
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

    #[tokio::test]
    async fn store_sql_events_persists_to_sqlite_worker() {
        let path = temporary_sqlite_path("event-persistence");
        let store = SqliteEventStore::open(&path).expect("sqlite store should open");
        let (persistence, worker) = EventPersistence::sqlite(store);
        let state = ApiState::default();
        let event_id = SqlEventId("evt_persisted".to_owned());

        store_sql_events(
            &state,
            &persistence,
            vec![test_event(
                event_id.clone(),
                CaptureStatus::Ok,
                DurationMillis(12),
            )],
        )
        .await;
        drop(persistence);
        worker
            .join()
            .expect("sqlite persistence worker should shut down");

        let reopened = SqliteEventStore::open(&path).expect("sqlite store should reopen");
        let row = reopened
            .get_event_row(&event_id)
            .expect("sqlite event lookup should succeed")
            .expect("event should be persisted");
        assert_eq!(row.id, "evt_persisted");
        assert_eq!(row.status, "ok");

        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn sqlite_worker_insert_failure_does_not_stop_capture_state() {
        let path = temporary_sqlite_path("event-persistence-failure");
        let store = SqliteEventStore::open(&path).expect("sqlite store should open");
        let (persistence, worker) = EventPersistence::sqlite(store);
        let state = ApiState::default();
        let event_id = SqlEventId("evt_duplicate".to_owned());
        let event = test_event(event_id.clone(), CaptureStatus::Ok, DurationMillis(12));

        store_sql_events(&state, &persistence, vec![event.clone(), event]).await;
        drop(persistence);
        worker
            .join()
            .expect("sqlite persistence worker should shut down after insert failure");

        let event_store = state.event_store();
        let store = event_store.read().await;
        let stats = store.stats();
        assert_eq!(stats.total_appended, 2);
        assert_eq!(stats.len, 2);
        drop(store);

        let live_statistics = state.live_statistics();
        let mut statistics = live_statistics.write().await;
        assert_eq!(statistics.snapshot().total_events, 2);

        let reopened = SqliteEventStore::open(&path).expect("sqlite store should reopen");
        assert!(
            reopened
                .get_event_row(&event_id)
                .expect("sqlite event lookup should succeed")
                .is_some()
        );

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn runtime_storage_from_ring_buffer_config_has_no_persistence_worker() {
        let storage = RuntimeStorage::from_config(&StorageConfig::default())
            .expect("default storage config should be valid");

        assert!(storage.persistence.sqlite_tx.is_none());
        assert!(storage.sqlite_worker.is_none());
    }

    fn temporary_sqlite_path(name: &str) -> PathBuf {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_millis();

        std::env::temp_dir().join(format!(
            "sql-lens-{name}-{}-{millis}.sqlite3",
            std::process::id()
        ))
    }

    fn test_event(id: SqlEventId, status: CaptureStatus, duration: DurationMillis) -> SqlEvent {
        SqlEvent {
            id,
            timestamp: Timestamp("2026-07-06T09:00:00Z".to_owned()),
            target_name: Some("mysql-local".to_owned()),
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
    NoProxyTargets,
    StorageConfig(String),
    SqliteStorage {
        path: PathBuf,
        source: Box<dyn Error + Send + Sync + 'static>,
    },
    SqlitePersistenceWorkerPanicked,
    Http(HttpServerError),
    ProxyListener(ProxyListenerError),
    BackendDial(BackendDialError),
    Join(JoinError),
}

impl fmt::Display for MinimalMysqlRuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoProxyTargets => write!(
                f,
                "minimal MySQL runtime requires at least one proxy target"
            ),
            Self::StorageConfig(message) => write!(f, "invalid runtime storage config: {message}"),
            Self::SqliteStorage { path, source } => {
                write!(
                    f,
                    "failed to initialize SQLite storage at {}: {source}",
                    path.display()
                )
            }
            Self::SqlitePersistenceWorkerPanicked => {
                write!(f, "SQLite persistence worker panicked")
            }
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
            Self::NoProxyTargets => None,
            Self::StorageConfig(_) => None,
            Self::SqliteStorage { source, .. } => Some(source.as_ref()),
            Self::SqlitePersistenceWorkerPanicked => None,
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
