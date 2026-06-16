use agent_core::prelude::*;
use agent_logging::initialise_logging;
use api_server::error::AppResult;
#[cfg(windows)]
use api_server::windows;
use api_server::{bootstrap_api_server, server_core::run_server_core};
use api_server::{LOGGING_WORKER_GUARDS, SERVICE_DISPLAY_NAME, SERVICE_NAME};
use clap::{Parser, Subcommand};
use tracing::{error, info};

#[derive(Parser)]
#[command(name = SERVICE_NAME)]
#[command(about = SERVICE_DISPLAY_NAME, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Install the Pebble Agent API Server as a Windows Service
    Install,
    /// Uninstall the Pebble Agent API Server from Windows Services
    Uninstall,
}

#[tokio::main]
async fn main() -> AppResult<()> {
    let cli = Cli::parse();

    if let Some(command) = cli.command {
        match command {
            Commands::Install => {
                #[cfg(windows)]
                {
                    if let Err(e) = windows::install_service() {
                        eprintln!("Failed to install service: {}", e);
                    }
                    return Ok(());
                }
                #[cfg(not(windows))]
                {
                    println!("The 'install' command is only supported on Windows targets.");
                    return Ok(());
                }
            }
            Commands::Uninstall => {
                #[cfg(windows)]
                {
                    if let Err(e) = windows::uninstall_service() {
                        eprintln!("Failed to uninstall service: {}", e);
                    }
                    return Ok(());
                }
                #[cfg(not(windows))]
                {
                    println!("The 'uninstall' command is only supported on Windows targets.");
                    return Ok(());
                }
            }
        }
    }

    //Initialise the runtime properties we will be leveraging
    RuntimeConstants::init("API_Server");
    let runtime_constants = RuntimeConstants::global();

    #[cfg(windows)]
    {
        if let Err(err) = windows::run() {
            eprintln!("Windows service startup failed: {err}");
        }
    }

    // Bootstrap the API Server
    // Return a set of bootstrap parameters we save and use
    let bootstrap_parameters = bootstrap_api_server()?;

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

    let (_shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    match run_server_core(bootstrap_parameters, shutdown_rx).await {
        Ok(()) => {}
        Err(error) => {
            error!(errorMsg=%error,"Error starting up API Server")
        }
    }

    Ok(())
}
