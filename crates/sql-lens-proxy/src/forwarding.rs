use crate::ProxiedConnection;
use std::{error::Error, fmt, io, net::SocketAddr};
use tokio::io::copy_bidirectional;

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
