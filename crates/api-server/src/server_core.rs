use crate::api::middleware::verify_jti_middleware;
use crate::api::v1::cache::cache_router;
use crate::api::v1::info::info_router;
use crate::api::v1::properties::properties_router;
use crate::api::v1::registration::registration_router;
use crate::error::{ApiError, AppResult};
use crate::scheduler::{setup_scheduler, ScheduledJobDependencies};
use crate::BootstrapParameters;
use agent_core::prelude::*;
use agent_database::RepositoryContainer;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::SqliteConnection;
use salvo::affix_state::inject;
use salvo::catcher::Catcher;
use salvo::http::response;
use salvo::oapi::security::{Http, HttpAuthScheme, SecurityScheme};
use salvo::prelude::*;
use std::sync::Arc;
use std::time::Duration;
use tokio_cron_scheduler::{Job, JobScheduler, JobSchedulerError};

#[handler]
async fn handle404(
    &self,
    req: &Request,
    _depot: &Depot,
    res: &mut Response,
    ctrl: &mut FlowCtrl,
) -> AppResult<()> {
    // Check if the error is a 404 Not Found
    if StatusCode::NOT_FOUND == res.status_code.unwrap_or(StatusCode::NOT_FOUND) {
        let endpoint = req.uri().path().to_string();
        return Err(ApiError::EndpointNotFoundError(endpoint));

        // // Return custom error page
        // res.render("Custom 404 Error Page");
        // // Skip remaining error handlers
        // ctrl.skip_rest();
    }
    Ok(())
}

pub async fn run_server_core(
    bootstrap_parameters: BootstrapParameters,
    shutdown_rx: tokio::sync::oneshot::Receiver<()>,
) -> AppResult<()> {
    // Save the scheduled job dependecies we need
    let job_dependencies = ScheduledJobDependencies {
        db_pool: bootstrap_parameters.db_pool.clone(),
        repos: bootstrap_parameters.repository_container.clone(),
    };

    //create the cron job scheduler
    let scheduler = setup_scheduler(job_dependencies).await?;
    scheduler.start().await?;

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
                .push(Router::with_path("api/v1").push(info_router()))
                .push(Router::with_path("api/v1").push(properties_router()))
                .push(Router::with_path("/api/v1").push(cache_router())),
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

    // Build service (router + catcher)
    let service = Service::new(router).catcher(Catcher::default().hoop(handle404));

    // Create server instance
    let server = Server::new(acceptor);

    // Get server handle for graceful shutdown
    let handle = server.handle();

    // Spawn the shutdown monitor task
    tokio::spawn(async move {
        // Wait for the OS wrapper (Windows SCM or Linux SIGTERM) to signal us
        if shutdown_rx.await.is_ok() {
            println!("OS termination signal received. Initiating Salvo graceful shutdown...");
            handle.stop_graceful(None);
        }
    });

    // Start serving
    server.serve(service).await;

    // Spawn the shutdown monitor task
    // tokio::spawn(async move {
    //     // Wait for the OS wrapper (Windows SCM or Linux SIGTERM) to signal us
    //     if let Ok(_) = shutdown_rx.await {
    //         println!("OS termination signal received. Initiating Salvo graceful shutdown...");
    //         // Stop gracefully, giving existing connections up to 10 seconds to finish
    //         handle.stop_graceful(Some(Duration::from_secs(10)));
    //     }
    // });

    // // Start the Salvo server. This blocks until `stop_graceful` finishes executing.
    // if let Err(e) = server.try_serve(router).await {
    //     eprintln!("Salvo server error: {}", e);
    // }

    // server.serve(router).await;

    // Print router structure for debugging
    // println!("{router:?}");

    // Start serving requests
    // let server = Server::new(acceptor).serve(router).await;
    // let
    Ok(())
}
