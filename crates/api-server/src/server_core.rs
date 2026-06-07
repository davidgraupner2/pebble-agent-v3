use crate::api::middleware::verify_jti_middleware;
use crate::api::v1::info::info_router;
use crate::api::v1::registration::registration_router;
use crate::BootstrapParameters;
use agent_core::prelude::*;
use salvo::affix_state::inject;
use salvo::oapi::security::{Http, HttpAuthScheme, SecurityScheme};
use salvo::prelude::*;
use std::time::Duration;

pub async fn run_server_core(
    bootstrap_parameters: BootstrapParameters,
    shutdown_rx: tokio::sync::oneshot::Receiver<()>,
) {
    // Start the listener on the loopback address and port
    // let tcp_listener_address = format!("127.0.0.1:{}", bootstrap_parameters.port);
    // let acceptor = TcpListener::new(tcp_listener_address).bind().await;

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
                .hoop(verify_jti_middleware)
                .push(Router::with_path("api/v1").push(info_router())),
        );

    let doc = OpenApi::new("Pebble Agent Api", "1.0.0")
        .merge_router(&router)
        .add_security_scheme(
            "bearer_token",
            SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer).bearer_format("JWT")),
        );
    let router = router
        .push(doc.into_router("/api-doc/openapi.json"))
        .push(SwaggerUi::new("/api-doc/openapi.json").into_router("/swagger-ui"))
        .push(ReDoc::new("/api-doc/openapi.json").into_router("/redoc"));

    let acceptor = TcpListener::new(format!("127.0.0.1:{}", bootstrap_parameters.port))
        .bind()
        .await;

    // Create server instance
    let server = Server::new(acceptor);

    // Get server handle for graceful shutdown
    let handle = server.handle();

    // Spawn the shutdown monitor task
    tokio::spawn(async move {
        // Wait for the OS wrapper (Windows SCM or Linux SIGTERM) to signal us
        if let Ok(_) = shutdown_rx.await {
            println!("OS termination signal received. Initiating Salvo graceful shutdown...");
            // Stop gracefully, giving existing connections up to 10 seconds to finish
            handle.stop_graceful(Some(Duration::from_secs(10)));
        }
    });

    // Start the Salvo server. This blocks until `stop_graceful` finishes executing.
    if let Err(e) = server.try_serve(router).await {
        eprintln!("Salvo server error: {}", e);
    }

    // server.serve(router).await;

    // Print router structure for debugging
    // println!("{router:?}");

    // Start serving requests
    // let server = Server::new(acceptor).serve(router).await;
    // let
}
