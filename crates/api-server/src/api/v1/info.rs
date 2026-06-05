use crate::api::extensions::DepotExt;
use agent_core::prelude::RegistrationClaims;
use salvo::prelude::*;
use salvo_jwt_auth::JwtAuthDepotExt;

#[endpoint]
async fn info(depot: &mut Depot) -> Result<String, StatusError> {
    let mut conn = depot.db_conn()?;

    if let Some(token_data) = depot.jwt_auth_data::<RegistrationClaims>() {
        Ok(format!("Welcome back, {:#?}!", token_data.claims))
    } else {
        Ok("Unauthorized access.".to_string())
    }

    // Ok("Hello World222".to_string())
}

pub fn info_router() -> Router {
    Router::with_path("info").get(info)
}
