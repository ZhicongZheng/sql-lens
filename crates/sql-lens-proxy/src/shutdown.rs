use sql_lens_config::ProxyConfig;
use std::{error::Error, fmt, time::Duration};
use tokio::{
    sync::{mpsc, watch},
    task::JoinHandle,
    time::{Instant, timeout_at},
};

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
