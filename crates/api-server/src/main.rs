use agent_core::prelude::*;
use agent_logging::initialise_logging;
use api_server::error::Result;
use api_server::{bootstrap_api_server, server_core::run_server_core, windows};
use api_server::{LOGGING_WORKER_GUARDS, SERVICE_DISPLAY_NAME, SERVICE_NAME};
use clap::{Parser, Subcommand};

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
async fn main() -> Result<()> {
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

    // Start the listener on the loopback address and port
    // let tcp_listener_address = format!("127.0.0.1:{}", bootstrap_parameters.port);
    // let acceptor = TcpListener::new(tcp_listener_address).bind().await;

    // // Create the API router
    // // Injecting the DB Pool, DB repositories and the Config struct for API Global use
    // let router = Router::new()
    //     .hoop(inject(bootstrap_parameters.db_pool))
    //     .hoop(inject(bootstrap_parameters.config))
    //     .hoop(inject(bootstrap_parameters.repository_container))
    //     // Non authenticated routes
    //     .push(
    //         Router::with_path("api/v1/")
    //             // .push(info_router())
    //             .push(registration_router()),
    //     )
    //     // Authenticated routes
    //     .push(
    //         Router::with_hoop(auth_jwt_middleware())
    //             .hoop(verify_jti_middleware)
    //             .push(Router::with_path("api/v1").push(info_router())),
    //     );

    // let doc = OpenApi::new("Pebble Agent Api", "1.0.0")
    //     .merge_router(&router)
    //     .add_security_scheme(
    //         "bearer_token",
    //         SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer).bearer_format("JWT")),
    //     );
    // let router = router
    //     .push(doc.into_router("/api-doc/openapi.json"))
    //     .push(SwaggerUi::new("/api-doc/openapi.json").into_router("/swagger-ui"))
    //     .push(ReDoc::new("/api-doc/openapi.json").into_router("/redoc"));

    // // Print router structure for debugging
    // println!("{router:?}");

    // // Start serving requests
    // Server::new(acceptor).serve(router).await;

    let (_shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    run_server_core(bootstrap_parameters, shutdown_rx).await;

    Ok(())
}
