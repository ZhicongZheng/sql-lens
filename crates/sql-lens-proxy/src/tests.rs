use super::*;
use sql_lens_config::{BackendConfig, ProxyConfig};
use sql_lens_core::{ConnectionId, ConnectionState, DatabaseType, ProtocolName, Timestamp};
use std::{
    io,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{mpsc, watch},
    time::{Duration, timeout},
};

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

fn test_timestamp(value: &str) -> Timestamp {
    Timestamp(value.to_owned())
}

fn test_lifecycle_record() -> ConnectionLifecycleRecord {
    ConnectionLifecycleRecord::accepted(
        ConnectionId("conn_test".to_owned()),
        Some("mysql-local".to_owned()),
        ProtocolName("mysql".to_owned()),
        DatabaseType("mysql".to_owned()),
        "127.0.0.1:51000",
        "127.0.0.1:3306",
        test_timestamp("accepted"),
    )
}

#[test]
fn lifecycle_id_generator_produces_sequential_connection_ids() {
    let generator = ConnectionLifecycleIdGenerator::new();

    let first = generator.next_id();
    let second = generator.next_id();
    let third = generator.next_id();

    assert_eq!(first, ConnectionId("conn_1".to_owned()));
    assert_eq!(second, ConnectionId("conn_2".to_owned()));
    assert_eq!(third, ConnectionId("conn_3".to_owned()));
}

#[test]
fn lifecycle_record_tracks_normal_forwarding_close() {
    let mut record = test_lifecycle_record();
    let summary = ForwardingSummary {
        client_peer_addr: "127.0.0.1:51000"
            .parse()
            .expect("client address should parse"),
        backend_address: "127.0.0.1:3306".to_owned(),
        client_to_backend_bytes: 128,
        backend_to_client_bytes: 256,
    };

    assert_eq!(record.info().state, ConnectionState::Created);

    record.mark_backend_connected(test_timestamp("backend-connected"));
    record.mark_forwarding_closed(&summary, test_timestamp("closed"));

    assert_eq!(record.info().state, ConnectionState::Closed);
    assert_eq!(record.info().bytes_in, 128);
    assert_eq!(record.info().bytes_out, 256);
    assert_eq!(record.info().closed_at, Some(test_timestamp("closed")));
    assert_eq!(
        record.info().last_activity_at,
        Some(test_timestamp("closed"))
    );
    assert_eq!(record.failure(), None);

    let states = record
        .transitions()
        .iter()
        .map(|transition| transition.state)
        .collect::<Vec<_>>();

    assert_eq!(
        states,
        vec![
            ConnectionState::Created,
            ConnectionState::BackendConnected,
            ConnectionState::Closing,
            ConnectionState::Closed,
        ]
    );
}

#[test]
fn lifecycle_record_tracks_backend_dial_failure() {
    let mut record = test_lifecycle_record();
    let failure = BackendDialFailure {
        client_peer_addr: "127.0.0.1:51000"
            .parse()
            .expect("client address should parse"),
        backend_address: "127.0.0.1:3306".to_owned(),
        kind: BackendDialFailureKind::Timeout {
            timeout: Duration::from_millis(50),
        },
    };

    record.mark_backend_dial_failed(&failure, test_timestamp("failed"));

    assert_eq!(record.info().state, ConnectionState::Failed);
    assert_eq!(record.info().closed_at, Some(test_timestamp("failed")));
    assert_eq!(
        record.info().last_activity_at,
        Some(test_timestamp("failed"))
    );
    assert_eq!(record.info().bytes_in, 0);
    assert_eq!(record.info().bytes_out, 0);
    assert_eq!(
        record.failure(),
        Some(&ConnectionLifecycleFailure {
            client_addr: "127.0.0.1:51000".to_owned(),
            backend_addr: "127.0.0.1:3306".to_owned(),
            kind: ConnectionLifecycleFailureKind::BackendDialTimeout {
                timeout: Duration::from_millis(50),
            },
        })
    );

    let states = record
        .transitions()
        .iter()
        .map(|transition| transition.state)
        .collect::<Vec<_>>();

    assert_eq!(
        states,
        vec![ConnectionState::Created, ConnectionState::Failed]
    );
}

#[test]
fn lifecycle_record_tracks_connection_limit_rejection() {
    let mut record = test_lifecycle_record();

    record.mark_connection_rejected(test_timestamp("rejected"));

    assert_eq!(record.info().state, ConnectionState::Failed);
    assert_eq!(record.info().closed_at, Some(test_timestamp("rejected")));
    assert_eq!(
        record.failure(),
        Some(&ConnectionLifecycleFailure {
            client_addr: "127.0.0.1:51000".to_owned(),
            backend_addr: "127.0.0.1:3306".to_owned(),
            kind: ConnectionLifecycleFailureKind::ConnectionLimit,
        })
    );
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
