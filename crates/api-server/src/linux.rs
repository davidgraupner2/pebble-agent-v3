use crate::bootstrap_api_server;
use crate::error::AppResult;
use crate::server_core::run_server_core;
use crate::LOGGING_WORKER_GUARDS;
use agent_core::prelude::RuntimeConstants;
use agent_logging::initialise_logging;
use tokio::signal::unix::{signal, SignalKind};
use tracing::{error, info};

pub async fn run() -> AppResult<()> {
    let bootstrap_parameters = bootstrap_api_server()?;
    let runtime_constants = RuntimeConstants::global();

    // Initialise logging
    let worker_guards = initialise_logging(
        runtime_constants.folders().logs(),
        runtime_constants.exe_name(),
        &bootstrap_parameters.logging_format,
        &bootstrap_parameters.logging_output,
        Some(&bootstrap_parameters.logging_level),
    );

    // Save the logging guards long term
    // This allows us to write logs for as long as the api runs
    LOGGING_WORKER_GUARDS.set(worker_guards).unwrap();

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    tokio::spawn(async move {
        let mut sigterm = signal(SignalKind::terminate()).expect("sigterm");
        let mut sigint = signal(SignalKind::interrupt()).expect("sigint");

        tokio::select! {
        _ = sigterm.recv() => {
        info!("SIGTERM received, shutting down...");
        }
        _ = sigint.recv() => {
        info!("SIGINT received, shutting down...");
        }
        }

        let _ = shutdown_tx.send(());
    });

    run_server_core(bootstrap_parameters, shutdown_rx).await;
    Ok(())
}
