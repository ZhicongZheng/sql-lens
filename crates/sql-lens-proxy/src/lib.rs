//! TCP proxy runtime for SQL Lens.

use sql_lens_config::{BackendConfig, ProxyConfig};
use std::{error::Error, fmt, future::Future, io, net::SocketAddr, time::Duration};
use tokio::{
    io::copy_bidirectional,
    net::{TcpListener, TcpStream},
    sync::{mpsc, watch},
    task::JoinHandle,
    time::{Instant, timeout, timeout_at},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProxyListenerConfig {
    pub listen: String,
}

impl ProxyListenerConfig {
    pub fn new(listen: impl Into<String>) -> Self {
        Self {
            listen: listen.into(),
        }
    }
}

#[derive(Debug)]
pub struct TcpProxyListener {
    listener: TcpListener,
}

impl TcpProxyListener {
    pub async fn bind(config: ProxyListenerConfig) -> Result<Self, ProxyListenerError> {
        let listener = TcpListener::bind(config.listen.as_str())
            .await
            .map_err(|source| ProxyListenerError::Bind {
                listen: config.listen,
                source,
            })?;

        let local_addr = listener
            .local_addr()
            .map_err(|source| ProxyListenerError::LocalAddr { source })?;
        tracing::info!(%local_addr, "TCP proxy listener bound");

        Ok(Self { listener })
    }

    pub fn local_addr(&self) -> Result<SocketAddr, ProxyListenerError> {
        self.listener
            .local_addr()
            .map_err(|source| ProxyListenerError::LocalAddr { source })
    }

    pub async fn accept(&self) -> Result<AcceptedClient, ProxyListenerError> {
        let (stream, peer_addr) = self
            .listener
            .accept()
            .await
            .map_err(|source| ProxyListenerError::Accept { source })?;

        Ok(AcceptedClient { peer_addr, stream })
    }

    pub async fn run_accept_loop(
        self,
        accepted_tx: mpsc::Sender<AcceptedClient>,
        mut shutdown: watch::Receiver<bool>,
    ) -> Result<AcceptLoopStats, ProxyListenerError> {
        let local_addr = self.local_addr()?;
        let mut stats = AcceptLoopStats::default();

        tracing::info!(%local_addr, "TCP proxy listener accepting connections");

        loop {
            if *shutdown.borrow_and_update() {
                tracing::info!(
                    %local_addr,
                    accepted_connections = stats.accepted_connections,
                    "TCP proxy listener stopped"
                );
                return Ok(stats);
            }

            tokio::select! {
                biased;

                changed = shutdown.changed() => {
                    match changed {
                        Ok(()) => {
                            if *shutdown.borrow_and_update() {
                                tracing::info!(
                                    %local_addr,
                                    accepted_connections = stats.accepted_connections,
                                    "TCP proxy listener stopped"
                                );
                                return Ok(stats);
                            }
                        }
                        Err(_) => {
                            tracing::info!(
                                %local_addr,
                                accepted_connections = stats.accepted_connections,
                                "TCP proxy listener shutdown sender dropped"
                            );
                            return Ok(stats);
                        }
                    }
                }
                accepted = self.accept() => {
                    let accepted = accepted?;
                    let peer_addr = accepted.peer_addr();

                    accepted_tx
                        .send(accepted)
                        .await
                        .map_err(|_| ProxyListenerError::AcceptedClientReceiverClosed)?;

                    stats.accepted_connections += 1;
                    tracing::debug!(
                        %local_addr,
                        %peer_addr,
                        accepted_connections = stats.accepted_connections,
                        "TCP proxy listener accepted client"
                    );
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct AcceptedClient {
    peer_addr: SocketAddr,
    stream: TcpStream,
}

impl AcceptedClient {
    pub fn peer_addr(&self) -> SocketAddr {
        self.peer_addr
    }

    pub fn into_stream(self) -> TcpStream {
        self.stream
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendDialConfig {
    pub address: String,
    pub connect_timeout: Duration,
}

impl BackendDialConfig {
    pub fn new(address: impl Into<String>, connect_timeout: Duration) -> Self {
        Self {
            address: address.into(),
            connect_timeout,
        }
    }

    pub fn from_config(proxy: &ProxyConfig, backend: &BackendConfig) -> Self {
        Self::new(
            backend.address.clone(),
            Duration::from_millis(proxy.connect_timeout_ms),
        )
    }
}

#[derive(Debug)]
pub struct BackendDialer;

impl BackendDialer {
    pub async fn dial(
        accepted: AcceptedClient,
        config: &BackendDialConfig,
    ) -> Result<ProxiedConnection, BackendDialError> {
        let backend_address = config.address.clone();
        let connect = TcpStream::connect(backend_address.clone());

        Self::dial_connecting(accepted, backend_address, config.connect_timeout, connect).await
    }

    async fn dial_connecting(
        accepted: AcceptedClient,
        backend_address: String,
        connect_timeout: Duration,
        connect: impl Future<Output = io::Result<TcpStream>>,
    ) -> Result<ProxiedConnection, BackendDialError> {
        let client_peer_addr = accepted.peer_addr;
        tracing::debug!(
            %client_peer_addr,
            backend_address = %backend_address,
            connect_timeout_ms = connect_timeout.as_millis(),
            "dialing backend"
        );

        match timeout(connect_timeout, connect).await {
            Ok(Ok(backend_stream)) => {
                tracing::debug!(
                    %client_peer_addr,
                    backend_address = %backend_address,
                    "backend dial succeeded"
                );

                Ok(ProxiedConnection {
                    client_stream: accepted.stream,
                    backend_stream,
                    client_peer_addr,
                    backend_address,
                })
            }
            Ok(Err(source)) => {
                let failure = BackendDialFailure {
                    client_peer_addr,
                    backend_address,
                    kind: BackendDialFailureKind::Connect,
                };

                Err(BackendDialError::Connect { failure, source })
            }
            Err(_) => {
                let failure = BackendDialFailure {
                    client_peer_addr,
                    backend_address,
                    kind: BackendDialFailureKind::Timeout {
                        timeout: connect_timeout,
                    },
                };

                Err(BackendDialError::Timeout { failure })
            }
        }
    }
}

#[derive(Debug)]
pub struct ProxiedConnection {
    client_stream: TcpStream,
    backend_stream: TcpStream,
    client_peer_addr: SocketAddr,
    backend_address: String,
}

impl ProxiedConnection {
    pub fn client_stream(&self) -> &TcpStream {
        &self.client_stream
    }

    pub fn backend_stream(&self) -> &TcpStream {
        &self.backend_stream
    }

    pub fn client_peer_addr(&self) -> SocketAddr {
        self.client_peer_addr
    }

    pub fn backend_address(&self) -> &str {
        &self.backend_address
    }

    pub fn into_parts(self) -> (TcpStream, TcpStream, SocketAddr, String) {
        (
            self.client_stream,
            self.backend_stream,
            self.client_peer_addr,
            self.backend_address,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendDialFailure {
    pub client_peer_addr: SocketAddr,
    pub backend_address: String,
    pub kind: BackendDialFailureKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendDialFailureKind {
    Timeout { timeout: Duration },
    Connect,
}

#[derive(Debug)]
pub enum BackendDialError {
    Timeout {
        failure: BackendDialFailure,
    },
    Connect {
        failure: BackendDialFailure,
        source: io::Error,
    },
}

impl BackendDialError {
    pub fn failure(&self) -> &BackendDialFailure {
        match self {
            Self::Timeout { failure } => failure,
            Self::Connect { failure, .. } => failure,
        }
    }
}

impl fmt::Display for BackendDialError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Timeout { failure } => {
                let BackendDialFailureKind::Timeout { timeout } = failure.kind else {
                    return write!(
                        f,
                        "backend dial timeout failure had non-timeout kind for backend {}",
                        failure.backend_address
                    );
                };

                write!(
                    f,
                    "backend dial to {} for client {} timed out after {:?}",
                    failure.backend_address, failure.client_peer_addr, timeout
                )
            }
            Self::Connect { failure, source } => write!(
                f,
                "failed to dial backend {} for client {}: {}",
                failure.backend_address, failure.client_peer_addr, source
            ),
        }
    }
}

impl Error for BackendDialError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Timeout { .. } => None,
            Self::Connect { source, .. } => Some(source),
        }
    }
}

#[derive(Debug)]
pub struct TcpForwarder;

impl TcpForwarder {
    pub async fn forward(
        connection: ProxiedConnection,
    ) -> Result<ForwardingSummary, ForwardingError> {
        let (mut client_stream, mut backend_stream, client_peer_addr, backend_address) =
            connection.into_parts();

        tracing::debug!(
            %client_peer_addr,
            backend_address = %backend_address,
            "TCP forwarding started"
        );

        match copy_bidirectional(&mut client_stream, &mut backend_stream).await {
            Ok((client_to_backend_bytes, backend_to_client_bytes)) => {
                tracing::debug!(
                    %client_peer_addr,
                    backend_address = %backend_address,
                    client_to_backend_bytes,
                    backend_to_client_bytes,
                    "TCP forwarding finished"
                );

                Ok(ForwardingSummary {
                    client_peer_addr,
                    backend_address,
                    client_to_backend_bytes,
                    backend_to_client_bytes,
                })
            }
            Err(source) => {
                let failure = ForwardingFailure {
                    client_peer_addr,
                    backend_address,
                    client_to_backend_bytes: None,
                    backend_to_client_bytes: None,
                };

                Err(ForwardingError::Io { failure, source })
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForwardingSummary {
    pub client_peer_addr: SocketAddr,
    pub backend_address: String,
    pub client_to_backend_bytes: u64,
    pub backend_to_client_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForwardingFailure {
    pub client_peer_addr: SocketAddr,
    pub backend_address: String,
    pub client_to_backend_bytes: Option<u64>,
    pub backend_to_client_bytes: Option<u64>,
}

#[derive(Debug)]
pub enum ForwardingError {
    Io {
        failure: ForwardingFailure,
        source: io::Error,
    },
}

impl ForwardingError {
    pub fn failure(&self) -> &ForwardingFailure {
        match self {
            Self::Io { failure, .. } => failure,
        }
    }
}

impl fmt::Display for ForwardingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { failure, source } => write!(
                f,
                "failed to forward TCP between client {} and backend {}: {}",
                failure.client_peer_addr, failure.backend_address, source
            ),
        }
    }
}

impl Error for ForwardingError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProxyShutdownConfig {
    pub drain_timeout: Duration,
}

impl ProxyShutdownConfig {
    pub fn new(drain_timeout: Duration) -> Self {
        Self { drain_timeout }
    }

    pub fn from_config(proxy: &ProxyConfig) -> Self {
        Self::new(Duration::from_millis(proxy.shutdown_timeout_ms))
    }
}

#[derive(Debug, Clone)]
pub struct ProxyShutdownSignal {
    sender: watch::Sender<bool>,
}

impl ProxyShutdownSignal {
    pub fn new() -> Self {
        let (sender, _receiver) = watch::channel(false);

        Self { sender }
    }

    pub fn subscribe(&self) -> watch::Receiver<bool> {
        self.sender.subscribe()
    }

    pub fn request_shutdown(&self) -> Result<(), ProxyShutdownError> {
        self.sender
            .send(true)
            .map_err(|_| ProxyShutdownError::NoReceivers)
    }
}

impl Default for ProxyShutdownSignal {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub enum ProxyShutdownError {
    NoReceivers,
}

impl fmt::Display for ProxyShutdownError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoReceivers => write!(f, "no proxy shutdown receivers are active"),
        }
    }
}

impl Error for ProxyShutdownError {}

#[derive(Debug)]
pub struct ActiveSessionDrain;

impl ActiveSessionDrain {
    pub async fn drain<T>(
        sessions: Vec<JoinHandle<T>>,
        config: &ProxyShutdownConfig,
    ) -> ShutdownDrainSummary
    where
        T: Send + 'static,
    {
        let total_sessions = sessions.len();

        if total_sessions == 0 {
            return ShutdownDrainSummary::default();
        }

        let abort_handles = sessions
            .iter()
            .map(JoinHandle::abort_handle)
            .collect::<Vec<_>>();
        let (status_tx, mut status_rx) = mpsc::channel(total_sessions);

        for session in sessions {
            let status_tx = status_tx.clone();
            tokio::spawn(async move {
                let status = match session.await {
                    Ok(_) => SessionDrainStatus::Completed,
                    Err(_) => SessionDrainStatus::Failed,
                };

                let _ = status_tx.send(status).await;
            });
        }
        drop(status_tx);

        let deadline = Instant::now() + config.drain_timeout;
        let mut summary = ShutdownDrainSummary::default();

        while summary.observed_sessions() < total_sessions {
            match timeout_at(deadline, status_rx.recv()).await {
                Ok(Some(SessionDrainStatus::Completed)) => summary.completed_sessions += 1,
                Ok(Some(SessionDrainStatus::Failed)) => summary.failed_sessions += 1,
                Ok(None) => break,
                Err(_) => {
                    let timed_out_sessions =
                        total_sessions.saturating_sub(summary.observed_sessions());

                    for abort_handle in abort_handles {
                        abort_handle.abort();
                    }

                    summary.timed_out_sessions = timed_out_sessions;
                    summary.timed_out = true;
                    return summary;
                }
            }
        }

        summary
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ShutdownDrainSummary {
    pub completed_sessions: usize,
    pub failed_sessions: usize,
    pub timed_out_sessions: usize,
    pub timed_out: bool,
}

impl ShutdownDrainSummary {
    pub fn observed_sessions(&self) -> usize {
        self.completed_sessions + self.failed_sessions + self.timed_out_sessions
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SessionDrainStatus {
    Completed,
    Failed,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AcceptLoopStats {
    pub accepted_connections: u64,
}

#[derive(Debug)]
pub enum ProxyListenerError {
    Bind { listen: String, source: io::Error },
    LocalAddr { source: io::Error },
    Accept { source: io::Error },
    AcceptedClientReceiverClosed,
}

impl fmt::Display for ProxyListenerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bind { listen, source } => {
                write!(f, "failed to bind TCP proxy listener {listen}: {source}")
            }
            Self::LocalAddr { source } => {
                write!(
                    f,
                    "failed to read TCP proxy listener local address: {source}"
                )
            }
            Self::Accept { source } => {
                write!(f, "failed to accept TCP proxy client connection: {source}")
            }
            Self::AcceptedClientReceiverClosed => {
                write!(f, "accepted client receiver closed")
            }
        }
    }
}

impl Error for ProxyListenerError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Bind { source, .. } => Some(source),
            Self::LocalAddr { source } => Some(source),
            Self::Accept { source } => Some(source),
            Self::AcceptedClientReceiverClosed => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::time::{Duration, timeout};

    const TEST_TIMEOUT: Duration = Duration::from_secs(1);

    async fn accept_test_client() -> (AcceptedClient, TcpStream) {
        let listener = TcpProxyListener::bind(ProxyListenerConfig::new("127.0.0.1:0"))
            .await
            .expect("listener should bind");
        let local_addr = listener.local_addr().expect("local address should exist");

        let client = TcpStream::connect(local_addr)
            .await
            .expect("client should connect");
        let accepted = listener.accept().await.expect("client should be accepted");

        (accepted, client)
    }

    async fn create_test_proxied_connection() -> (ProxiedConnection, TcpStream, TcpStream) {
        let backend_listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("backend listener should bind");
        let backend_addr = backend_listener
            .local_addr()
            .expect("backend local address should exist");
        let backend_accept = tokio::spawn(async move { backend_listener.accept().await });

        let (accepted, client) = accept_test_client().await;
        let config = BackendDialConfig::new(backend_addr.to_string(), TEST_TIMEOUT);
        let proxied = timeout(TEST_TIMEOUT, BackendDialer::dial(accepted, &config))
            .await
            .expect("backend dial should complete before timeout")
            .expect("backend dial should succeed");
        let (backend, _backend_peer_addr) = timeout(TEST_TIMEOUT, backend_accept)
            .await
            .expect("backend accept task should complete before timeout")
            .expect("backend accept task should join")
            .expect("backend should accept the dialed connection");

        (proxied, client, backend)
    }

    async fn finish_forwarding(
        forward: tokio::task::JoinHandle<Result<ForwardingSummary, ForwardingError>>,
    ) -> ForwardingSummary {
        timeout(TEST_TIMEOUT, forward)
            .await
            .expect("forwarding should finish before timeout")
            .expect("forwarding task should join")
            .expect("forwarding should finish cleanly")
    }

    struct DropFlag {
        dropped: Arc<AtomicBool>,
    }

    impl Drop for DropFlag {
        fn drop(&mut self) {
            self.dropped.store(true, Ordering::SeqCst);
        }
    }

    #[test]
    fn proxy_shutdown_config_uses_runtime_config() {
        let proxy = ProxyConfig {
            shutdown_timeout_ms: 2_500,
            ..ProxyConfig::default()
        };

        let config = ProxyShutdownConfig::from_config(&proxy);

        assert_eq!(config.drain_timeout, Duration::from_millis(2_500));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn shutdown_signal_notifies_subscribers() {
        let signal = ProxyShutdownSignal::new();
        let mut shutdown = signal.subscribe();

        assert!(!*shutdown.borrow_and_update());

        signal
            .request_shutdown()
            .expect("shutdown should notify active receivers");

        shutdown
            .changed()
            .await
            .expect("shutdown receiver should observe change");

        assert!(*shutdown.borrow_and_update());
    }

    #[test]
    fn shutdown_signal_reports_missing_receivers() {
        let signal = ProxyShutdownSignal::new();

        let error = signal
            .request_shutdown()
            .expect_err("shutdown without receivers should fail");

        assert!(matches!(error, ProxyShutdownError::NoReceivers));
        assert!(!error.to_string().is_empty());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn active_session_drain_reports_completed_sessions() {
        let config = ProxyShutdownConfig::new(TEST_TIMEOUT);
        let sessions = vec![tokio::spawn(async { 1_u8 }), tokio::spawn(async { 2_u8 })];

        let summary = ActiveSessionDrain::drain(sessions, &config).await;

        assert_eq!(
            summary,
            ShutdownDrainSummary {
                completed_sessions: 2,
                failed_sessions: 0,
                timed_out_sessions: 0,
                timed_out: false,
            }
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn active_session_drain_reports_failed_sessions() {
        let config = ProxyShutdownConfig::new(TEST_TIMEOUT);
        let session = tokio::spawn(async {
            std::future::pending::<()>().await;
        });
        session.abort();
        let sessions = vec![session];

        let summary = ActiveSessionDrain::drain(sessions, &config).await;

        assert_eq!(
            summary,
            ShutdownDrainSummary {
                completed_sessions: 0,
                failed_sessions: 1,
                timed_out_sessions: 0,
                timed_out: false,
            }
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn active_session_drain_times_out_and_aborts_unfinished_sessions() {
        let config = ProxyShutdownConfig::new(Duration::from_millis(10));
        let dropped = Arc::new(AtomicBool::new(false));
        let dropped_in_task = Arc::clone(&dropped);
        let sessions = vec![tokio::spawn(async move {
            let _drop_flag = DropFlag {
                dropped: dropped_in_task,
            };
            std::future::pending::<()>().await;
        })];

        let summary = ActiveSessionDrain::drain(sessions, &config).await;

        for _ in 0..10 {
            if dropped.load(Ordering::SeqCst) {
                break;
            }
            tokio::task::yield_now().await;
        }

        assert_eq!(
            summary,
            ShutdownDrainSummary {
                completed_sessions: 0,
                failed_sessions: 0,
                timed_out_sessions: 1,
                timed_out: true,
            }
        );
        assert!(dropped.load(Ordering::SeqCst));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn forwarding_copies_client_to_backend() {
        let (proxied, mut client, mut backend) = create_test_proxied_connection().await;
        let client_peer_addr = proxied.client_peer_addr();
        let backend_address = proxied.backend_address().to_owned();
        let forward = tokio::spawn(TcpForwarder::forward(proxied));
        let payload = b"client says hello";
        let mut received = vec![0; payload.len()];

        client
            .write_all(payload)
            .await
            .expect("client should write payload");
        client.shutdown().await.expect("client should shutdown");
        backend
            .read_exact(&mut received)
            .await
            .expect("backend should read forwarded payload");
        backend.shutdown().await.expect("backend should shutdown");

        let summary = finish_forwarding(forward).await;

        assert_eq!(received, payload);
        assert_eq!(summary.client_peer_addr, client_peer_addr);
        assert_eq!(summary.backend_address, backend_address);
        assert_eq!(summary.client_to_backend_bytes, payload.len() as u64);
        assert_eq!(summary.backend_to_client_bytes, 0);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn forwarding_copies_backend_to_client() {
        let (proxied, mut client, mut backend) = create_test_proxied_connection().await;
        let client_peer_addr = proxied.client_peer_addr();
        let backend_address = proxied.backend_address().to_owned();
        let forward = tokio::spawn(TcpForwarder::forward(proxied));
        let payload = b"backend says hello";
        let mut received = vec![0; payload.len()];

        backend
            .write_all(payload)
            .await
            .expect("backend should write payload");
        backend.shutdown().await.expect("backend should shutdown");
        client
            .read_exact(&mut received)
            .await
            .expect("client should read forwarded payload");
        client.shutdown().await.expect("client should shutdown");

        let summary = finish_forwarding(forward).await;

        assert_eq!(received, payload);
        assert_eq!(summary.client_peer_addr, client_peer_addr);
        assert_eq!(summary.backend_address, backend_address);
        assert_eq!(summary.client_to_backend_bytes, 0);
        assert_eq!(summary.backend_to_client_bytes, payload.len() as u64);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn forwarding_reports_bidirectional_byte_counts() {
        let (proxied, mut client, mut backend) = create_test_proxied_connection().await;
        let forward = tokio::spawn(TcpForwarder::forward(proxied));
        let client_payload = b"select 1";
        let backend_payload = b"ok";
        let mut backend_received = vec![0; client_payload.len()];
        let mut client_received = vec![0; backend_payload.len()];

        client
            .write_all(client_payload)
            .await
            .expect("client should write payload");
        backend
            .write_all(backend_payload)
            .await
            .expect("backend should write payload");

        backend
            .read_exact(&mut backend_received)
            .await
            .expect("backend should read forwarded client payload");
        client
            .read_exact(&mut client_received)
            .await
            .expect("client should read forwarded backend payload");

        client.shutdown().await.expect("client should shutdown");
        backend.shutdown().await.expect("backend should shutdown");

        let summary = finish_forwarding(forward).await;

        assert_eq!(backend_received, client_payload);
        assert_eq!(client_received, backend_payload);
        assert_eq!(summary.client_to_backend_bytes, client_payload.len() as u64);
        assert_eq!(
            summary.backend_to_client_bytes,
            backend_payload.len() as u64
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn forwarding_propagates_close_and_finishes_cleanly() {
        let (proxied, mut client, mut backend) = create_test_proxied_connection().await;
        let forward = tokio::spawn(TcpForwarder::forward(proxied));
        let mut eof_probe = [0_u8; 1];

        client.shutdown().await.expect("client should shutdown");

        let read = timeout(TEST_TIMEOUT, backend.read(&mut eof_probe))
            .await
            .expect("backend should observe propagated close before timeout")
            .expect("backend read should succeed");

        backend.shutdown().await.expect("backend should shutdown");

        let summary = finish_forwarding(forward).await;

        assert_eq!(read, 0);
        assert_eq!(summary.client_to_backend_bytes, 0);
        assert_eq!(summary.backend_to_client_bytes, 0);
    }

    #[test]
    fn backend_dial_config_uses_runtime_config() {
        let proxy = ProxyConfig {
            connect_timeout_ms: 1_234,
            ..ProxyConfig::default()
        };
        let backend = BackendConfig {
            address: "127.0.0.1:4406".to_owned(),
            ..BackendConfig::default()
        };

        let config = BackendDialConfig::from_config(&proxy, &backend);

        assert_eq!(config.address, "127.0.0.1:4406");
        assert_eq!(config.connect_timeout, Duration::from_millis(1_234));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn backend_dial_succeeds() {
        let backend_listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("backend listener should bind");
        let backend_addr = backend_listener
            .local_addr()
            .expect("backend local address should exist");
        let backend_accept = tokio::spawn(async move { backend_listener.accept().await });
        let (accepted, client) = accept_test_client().await;
        let client_peer_addr = accepted.peer_addr();
        let config = BackendDialConfig::new(backend_addr.to_string(), TEST_TIMEOUT);

        let proxied = timeout(TEST_TIMEOUT, BackendDialer::dial(accepted, &config))
            .await
            .expect("backend dial should complete before test timeout")
            .expect("backend dial should succeed");
        let (_backend_side, _backend_peer_addr) = timeout(TEST_TIMEOUT, backend_accept)
            .await
            .expect("backend accept task should complete before timeout")
            .expect("backend accept task should join")
            .expect("backend should accept the dialed connection");

        assert_eq!(proxied.client_peer_addr(), client_peer_addr);
        assert_eq!(proxied.backend_address(), backend_addr.to_string());
        assert!(
            proxied
                .backend_stream()
                .peer_addr()
                .expect("backend peer address should exist")
                .ip()
                .is_loopback()
        );

        drop(proxied);
        drop(client);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn backend_dial_failure_is_structured() {
        let unused_listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("unused listener should bind");
        let backend_addr = unused_listener
            .local_addr()
            .expect("unused listener local address should exist");
        drop(unused_listener);

        let (accepted, client) = accept_test_client().await;
        let client_peer_addr = accepted.peer_addr();
        let config = BackendDialConfig::new(backend_addr.to_string(), TEST_TIMEOUT);

        let error = timeout(TEST_TIMEOUT, BackendDialer::dial(accepted, &config))
            .await
            .expect("failed backend dial should complete before test timeout")
            .expect_err("backend dial should fail");

        match error {
            BackendDialError::Connect { failure, source } => {
                assert_eq!(failure.client_peer_addr, client_peer_addr);
                assert_eq!(failure.backend_address, backend_addr.to_string());
                assert_eq!(failure.kind, BackendDialFailureKind::Connect);
                assert_ne!(source.kind(), io::ErrorKind::TimedOut);
            }
            other => panic!("expected connect error, got {other:?}"),
        }

        drop(client);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn backend_dial_timeout_is_structured() {
        let (accepted, client) = accept_test_client().await;
        let client_peer_addr = accepted.peer_addr();
        let backend_address = "127.0.0.1:3306".to_owned();

        let error = BackendDialer::dial_connecting(
            accepted,
            backend_address.clone(),
            Duration::ZERO,
            std::future::pending(),
        )
        .await
        .expect_err("pending backend dial should fail when timeout elapses");

        match error {
            BackendDialError::Timeout { failure } => {
                assert_eq!(failure.client_peer_addr, client_peer_addr);
                assert_eq!(failure.backend_address, backend_address);
                assert_eq!(
                    failure.kind,
                    BackendDialFailureKind::Timeout {
                        timeout: Duration::ZERO
                    }
                );
            }
            other => panic!("expected timeout error, got {other:?}"),
        }

        drop(client);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn listener_binds_configured_address() {
        let listener = TcpProxyListener::bind(ProxyListenerConfig::new("127.0.0.1:0"))
            .await
            .expect("listener should bind");

        let local_addr = listener.local_addr().expect("local address should exist");

        assert!(local_addr.ip().is_loopback());
        assert_ne!(local_addr.port(), 0);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn bind_failure_returns_structured_error() {
        let listener = TcpProxyListener::bind(ProxyListenerConfig::new("127.0.0.1:0"))
            .await
            .expect("first listener should bind");
        let listen = listener
            .local_addr()
            .expect("local address should exist")
            .to_string();

        let error = TcpProxyListener::bind(ProxyListenerConfig::new(listen.clone()))
            .await
            .expect_err("second listener on same address should fail");

        match error {
            ProxyListenerError::Bind {
                listen: error_listen,
                source,
            } => {
                assert_eq!(error_listen, listen);
                assert_eq!(source.kind(), io::ErrorKind::AddrInUse);
            }
            other => panic!("expected bind error, got {other:?}"),
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn accept_loop_delivers_client_connection() {
        let listener = TcpProxyListener::bind(ProxyListenerConfig::new("127.0.0.1:0"))
            .await
            .expect("listener should bind");
        let local_addr = listener.local_addr().expect("local address should exist");
        let (accepted_tx, mut accepted_rx) = mpsc::channel(1);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let accept_loop = tokio::spawn(listener.run_accept_loop(accepted_tx, shutdown_rx));
        let client = TcpStream::connect(local_addr)
            .await
            .expect("client should connect");

        let accepted = timeout(TEST_TIMEOUT, accepted_rx.recv())
            .await
            .expect("accepted client should arrive before timeout")
            .expect("accepted client channel should stay open");

        assert!(accepted.peer_addr().ip().is_loopback());

        shutdown_tx.send(true).expect("shutdown should send");

        let stats = timeout(TEST_TIMEOUT, accept_loop)
            .await
            .expect("accept loop should stop before timeout")
            .expect("accept loop task should join")
            .expect("accept loop should stop cleanly");

        assert_eq!(
            stats,
            AcceptLoopStats {
                accepted_connections: 1
            }
        );

        drop(accepted);
        drop(client);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn accept_loop_stops_on_shutdown_without_connection() {
        let listener = TcpProxyListener::bind(ProxyListenerConfig::new("127.0.0.1:0"))
            .await
            .expect("listener should bind");
        let (accepted_tx, _accepted_rx) = mpsc::channel(1);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let accept_loop = tokio::spawn(listener.run_accept_loop(accepted_tx, shutdown_rx));

        shutdown_tx.send(true).expect("shutdown should send");

        let stats = timeout(TEST_TIMEOUT, accept_loop)
            .await
            .expect("accept loop should stop before timeout")
            .expect("accept loop task should join")
            .expect("accept loop should stop cleanly");

        assert_eq!(
            stats,
            AcceptLoopStats {
                accepted_connections: 0
            }
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn accept_loop_stops_with_proxy_shutdown_signal() {
        let listener = TcpProxyListener::bind(ProxyListenerConfig::new("127.0.0.1:0"))
            .await
            .expect("listener should bind");
        let (accepted_tx, _accepted_rx) = mpsc::channel(1);
        let shutdown = ProxyShutdownSignal::new();
        let accept_loop = tokio::spawn(listener.run_accept_loop(accepted_tx, shutdown.subscribe()));

        shutdown
            .request_shutdown()
            .expect("shutdown should notify listener");

        let stats = timeout(TEST_TIMEOUT, accept_loop)
            .await
            .expect("accept loop should stop before timeout")
            .expect("accept loop task should join")
            .expect("accept loop should stop cleanly");

        assert_eq!(
            stats,
            AcceptLoopStats {
                accepted_connections: 0
            }
        );
    }
}
