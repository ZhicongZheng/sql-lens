use std::{error::Error, fmt, io, net::SocketAddr};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{mpsc, watch},
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
