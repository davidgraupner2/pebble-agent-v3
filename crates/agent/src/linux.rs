use crate::agent_core::run_agent_core;
use crate::proxy::ProxySetting;
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
    connection_string: String,
    connection_timeout: u16,
    ping_interval: u16,
    retry_interval: u16,
    pong_response_interval: u16,
    proxy_settings: ProxySetting,
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
        connection_string,
        connection_timeout,
        ping_interval,
        retry_interval,
        pong_response_interval,
        proxy_settings,
        shutdown_rx,
    )
    .await
}
