use crate::actors::controller::{actor::Controller, arguments::ControllerArguments};
use anyhow::Result;
use ractor::Actor;
use std::time::Duration;
use tokio::sync::watch;
use tracing::info;

pub async fn run_agent_core(
    standalone: bool,
    api_host: String,
    api_port: u16,
    log_format: String,
    log_output: String,
    logging_level: String,
    mut shutdown_rx: watch::Receiver<bool>,
) -> Result<()> {
    let agent_runtime_controller_arguments = ControllerArguments {
        standalone,
        api_host,
        api_port,
        log_format,
        log_output,
        logging_level,
    };

    // Start the runtime controller
    let (controller, _actor_handle) = Actor::spawn(
        Some("AgentRuntimeController".to_string()),
        Controller,
        agent_runtime_controller_arguments,
    )
    .await
    .expect("Agent RuntimeController failed to start");

    // Foreground wait: keeps process alive until shutdown requested.
    loop {
        if *shutdown_rx.borrow() {
            break;
        }
        if shutdown_rx.changed().await.is_err() {
            // Sender dropped -> treat as shutdown
            break;
        }
    }

    info!("Shutdown requested. Stopping AgentRuntimeController...");
    let _ = controller
        .stop_and_wait(
            Some("Shutdown requested".to_string()),
            Some(Duration::from_secs(30)),
        )
        .await;

    Ok(())
}
