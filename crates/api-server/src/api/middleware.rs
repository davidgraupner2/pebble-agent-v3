// use crate::api::extensions::DepotExt;
// use agent_core::prelude::RegistrationClaims;
// use salvo::http::StatusCode;
// use salvo::prelude::*;
// use salvo_jwt_auth::JwtAuthDepotExt;

// #[handler]
// pub async fn verify_jti_middleware(
//     depot: &mut Depot,
//     res: &mut Response,
//     req: &mut Request,
//     ctrl: &mut FlowCtrl,
// ) {
//     // 1. Retrieve the successfully parsed JWT claims
//     let jwt_claims = depot.jwt_auth_data::<RegistrationClaims>();
//     if jwt_claims.is_none() {
//         res.status_code(StatusCode::UNAUTHORIZED);
//         res.render("Unauthorized: Invalid token structure.");
//     }

//     let token_jti = jwt_claims.unwrap().claims.jti.clone();

//     // Extract the database connection and database repositories from depot
//     let db_connection = depot.db_conn();
//     let repositories = depot.repositories();

//     if db_connection.is_err() {
//         res.status_code(StatusCode::UNAUTHORIZED);
//         res.render("Unauthorized: Database access error.");
//     }

//     if repositories.is_err() {
//         res.status_code(StatusCode::UNAUTHORIZED);
//         res.render("Unauthorized: Database access error.");
//     }

//     let jti_record = repositories
//         .unwrap()
//         .agent_jwt_repo
//         .get_by_jti(&mut db_connection.unwrap(), &token_jti);

//     if jti_record.is_err() {
//         res.status_code(StatusCode::UNAUTHORIZED);
//         res.render("Unauthorized: JTI could not be validated");
//     }

//     let jti_record = jti_record.unwrap();
//     if jti_record.is_none() {
//         res.status_code(StatusCode::UNAUTHORIZED);
//         res.render("Unauthorized: JTI is not activated");
//     };

//     ctrl.call_next(req, depot, res).await;
// }

use crate::api::extensions::DepotExt;
use agent_core::prelude::RegistrationClaims;
use salvo::http::StatusCode;
use salvo::prelude::*;
use salvo_jwt_auth::JwtAuthDepotExt;
use tracing::{error, warn};

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
