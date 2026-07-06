use crate::AcceptedClient;
use sql_lens_config::{BackendConfig, ProxyConfig};
use std::{error::Error, fmt, future::Future, io, net::SocketAddr, time::Duration};
use tokio::{net::TcpStream, time::timeout};

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

    pub(crate) async fn dial_connecting(
        accepted: AcceptedClient,
        backend_address: String,
        connect_timeout: Duration,
        connect: impl Future<Output = io::Result<TcpStream>>,
    ) -> Result<ProxiedConnection, BackendDialError> {
        let client_peer_addr = accepted.peer_addr();
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
                    client_stream: accepted.into_stream(),
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
