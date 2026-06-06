use crate::api::extensions::DepotExt;
use agent_core::prelude::RegistrationClaims;
use salvo::http::StatusCode;
use salvo::prelude::*;
use salvo_jwt_auth::JwtAuthDepotExt;
use tracing::{error, warn};

/// JWT JTI Verification Middleware
///
/// Validates that a JWT's JTI (JWT ID) claim exists and is active in the database.
/// This middleware runs after initial JWT signature validation and checks the tracking table
/// to ensure the token hasn't been revoked. When new JWTs are issued, all previous tokens
/// are automatically deactivated.
#[handler]
pub async fn verify_jti_middleware(
    depot: &mut Depot,
    res: &mut Response,
    req: &mut Request,
    ctrl: &mut FlowCtrl,
) {
    // 1) Claims must already be parsed by JWT middleware.
    let token_data = match depot.jwt_auth_data::<RegistrationClaims>() {
        Some(data) => data,
        None => {
            warn!("jwt claims missing from depot");
            unauthorized(res);
            return;
        }
    };

    let token_jti = &token_data.claims.jti;

    // 2) Resolve dependencies from depot.
    let mut conn = match depot.db_conn() {
        Ok(conn) => conn,
        Err(e) => {
            error!(error = %e, "failed to acquire db connection");
            internal_error(res);
            return;
        }
    };

    let repositories = match depot.repositories() {
        Ok(repos) => repos,
        Err(e) => {
            error!(error = %e, "failed to resolve repository container");
            internal_error(res);
            return;
        }
    };

    // 3) Validate JTI against active token records.
    let jti_record = match repositories.agent_jwt_repo.get_by_jti(&mut conn, token_jti) {
        Ok(record) => record,
        Err(e) => {
            error!(error = %e, jti = %token_jti, "jti lookup failed");
            internal_error(res);
            return;
        }
    };

    if jti_record.is_none() {
        warn!(jti = %token_jti, "jti not found or inactive");
        unauthorized(res);
        return;
    }

    // 4) Continue only after all checks pass.
    ctrl.call_next(req, depot, res).await;
}

fn unauthorized(res: &mut Response) {
    res.status_code(StatusCode::UNAUTHORIZED);
    res.render("Unauthorized");
}

fn internal_error(res: &mut Response) {
    res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
    res.render("Internal server error");
}
