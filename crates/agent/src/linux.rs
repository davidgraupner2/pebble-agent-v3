use crate::agent_core::run_agent_core;
use anyhow::Result;
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::watch;
use tracing::info;

pub async fn run_linux(
    standalone: bool,
    api_host: String,
    api_port: u16,
    log_format: String,
    log_output: String,
    logging_level: String,
) -> Result<()> {
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    // Linux lifecycle bridge: convert SIGTERM/SIGINT into shared shutdown signal.
    tokio::spawn(async move {
        let mut sigterm = signal(SignalKind::terminate()).expect("failed to register SIGTERM");
        let mut sigint = signal(SignalKind::interrupt()).expect("failed to register SIGINT");

        tokio::select! {
            _ = sigterm.recv() => info!("SIGTERM received, requesting shutdown"),
            _ = sigint.recv() => info!("SIGINT received, requesting shutdown"),
        }

        let _ = shutdown_tx.send(true);
    });

    run_agent_core(
        standalone,
        api_host,
        api_port,
        log_format,
        log_output,
        logging_level,
        shutdown_rx,
    )
    .await
}
