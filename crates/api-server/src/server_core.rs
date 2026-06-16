use crate::api::middleware::verify_jti_middleware;
use crate::api::v1::cache::cache_router;
use crate::api::v1::info::info_router;
use crate::api::v1::properties::properties_router;
use crate::api::v1::registration::registration_router;
use crate::error::{ApiError, AppResult};
use crate::scheduler::{setup_scheduler, ScheduledJobDependencies};
use crate::BootstrapParameters;
use agent_core::prelude::*;
use salvo::affix_state::inject;
use salvo::catcher::Catcher;
use salvo::oapi::security::{Http, HttpAuthScheme, SecurityScheme};
use salvo::prelude::*;
use salvo_jwt_auth::JwtAuthState::Unauthorized;
use tracing::{error, info};

#[handler]
async fn handle_server_errors(
    &self,
    req: &Request,
    _depot: &Depot,
    res: &mut Response,
    _ctrl: &mut FlowCtrl,
) -> AppResult<()> {
    // Check if the error is a 404 Not Found
    if StatusCode::NOT_FOUND == res.status_code.unwrap_or(StatusCode::NOT_FOUND) {
        let endpoint = req.uri().path().to_string();
        return Err(ApiError::EndpointNotFoundError(endpoint));
    } else if StatusCode::UNAUTHORIZED == res.status_code.unwrap_or(StatusCode::UNAUTHORIZED) {
        return Err(ApiError::AuthorisationError(
            "Bearer token not found".to_string(),
        ));
    } else if StatusCode::METHOD_NOT_ALLOWED
        == res.status_code.unwrap_or(StatusCode::METHOD_NOT_ALLOWED)
    {
        return Err(ApiError::BadRequest("HTTP Method not allowed".to_string()));
    }
    Ok(())
}

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
    }
    Ok(())
}

#[handler]
async fn handle_unauthorised(
    &self,
    _req: &Request,
    _depot: &Depot,
    res: &mut Response,
    _ctrl: &mut FlowCtrl,
) -> AppResult<()> {
    // Check if the error is a 401 Unauthorised
    if StatusCode::UNAUTHORIZED == res.status_code.unwrap_or(StatusCode::UNAUTHORIZED) {
        return Err(ApiError::AuthorisationError(
            "Bearer token not found".to_string(),
        ));
    }
    Ok(())
}

pub async fn run_server_core(
    bootstrap_parameters: BootstrapParameters,
    shutdown_rx: tokio::sync::oneshot::Receiver<()>,
) -> AppResult<()> {
    // Store the scheduled job dependecies we need
    // The DB Pool and the DB Repositories
    let job_dependencies = ScheduledJobDependencies {
        db_pool: bootstrap_parameters.db_pool.clone(),
        repos: bootstrap_parameters.repository_container.clone(),
        config: bootstrap_parameters.config.clone(),
    };

    //Setup and start the job scheduler
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
    let service = Service::new(router).catcher(Catcher::default().hoop(handle_server_errors));
    // .catcher(Catcher::default().hoop(handle404))
    // .catcher(Catcher::default().hoop(handle_unauthorised));

    // Create server instance
    let server = Server::new(acceptor);

    // Get server handle for graceful shutdown
    let handle = server.handle();

    // Spawn the shutdown monitor task
    tokio::spawn(async move {
        // Wait for the OS wrapper (Windows SCM or Linux SIGTERM) to signal us
        if shutdown_rx.await.is_ok() {
            println!("OS termination signal received. Initiating API Server shutdown...");
            handle.stop_graceful(None);
        }
    });

    // Start serving
    server.serve(service).await;

    Ok(())
}
