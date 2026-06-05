use agent_core::prelude::*;
use agent_logging::initialise_logging;
use api_server::api::v1::info::info_router;
use api_server::api::v1::registration::registration_router;
use api_server::bootstrap_api_server;
use api_server::error::Result;
use api_server::LOGGING_WORKER_GUARDS;
use salvo::affix_state::inject;
use salvo::prelude::*;

#[handler]
async fn info() -> &'static str {
    "Hello World"
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging subsystem
    // tracing_subscriber::fmt().init();

    // Initialise the runtime properties we will be leveraging
    RuntimeConstants::init("API_Server");
    let runtime_constants = RuntimeConstants::global();

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
    let tcp_listener_address = format!("127.0.0.1:{}", bootstrap_parameters.port);
    let acceptor = TcpListener::new(tcp_listener_address).bind().await;

    // Create the API router
    // Injecting the DB Pool, DB repositories and the Config struct for API Global use
    let router = Router::new()
        .hoop(inject(bootstrap_parameters.db_pool))
        .hoop(inject(bootstrap_parameters.config))
        .hoop(inject(bootstrap_parameters.repository_container))
        // Non authenticated routes
        .push(
            Router::with_path("api/v1/")
                // .push(info_router())
                .push(registration_router()),
        )
        // Authenticated routes
        .push(
            Router::with_hoop(auth_jwt_middleware())
                .push(Router::with_path("api/v1").push(info_router())),
        );

    let doc = OpenApi::new("Pebble Agent Api", "1.0.0").merge_router(&router);
    let router = router
        .push(doc.into_router("/api-doc/openapi.json"))
        .push(SwaggerUi::new("/api-doc/openapi.json").into_router("/swagger-ui"))
        .push(ReDoc::new("/api-doc/openapi.json").into_router("/redoc"));

    // Print router structure for debugging
    println!("{router:?}");

    // Start serving requests
    Server::new(acceptor).serve(router).await;

    Ok(())
}
