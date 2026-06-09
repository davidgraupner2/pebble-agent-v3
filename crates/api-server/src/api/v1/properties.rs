use crate::api::extensions::DepotExt;
use crate::error::{ApiError, Result};
use agent_core::prelude::*;
use agent_database::{RepositoryGetSet, TypedProperty};
use salvo::oapi::extract::{PathParam, QueryParam};
use salvo::prelude::*;
use salvo_jwt_auth::JwtAuthDepotExt;
use tracing::{debug, error, info, warn};

pub fn properties_router() -> Router {
    Router::with_path("property/{name}").get(get_property)
}

/// Retrieve a persisted property.
///
/// Why this endpoint exists:
/// - Agents need to read effective runtime settings from durable storage.
/// - Property lookup is scoped to the caller identity, preventing cross-agent reads.
///
/// How it works:
/// - The caller supplies the property `name` in the route path.
/// - The API resolves the caller's `registration_id` from the validated bearer token.
/// - The property repository performs a scoped lookup by `(name, registration_id)`.
///
/// Response behavior:
/// - `200`: property found and returned as a typed payload.
/// - `404`: no property exists for that name within the caller scope.
/// - `500`: unexpected storage or server failure.
///
/// Security notes:
/// - Requires a valid bearer JWT.
/// - Data access is tenant-scoped by registration identity.
#[endpoint(security(("bearer_token"=[])),tags("Properties"), status_codes(200, 500, 404))]
async fn get_property(depot: &mut Depot, name: PathParam<String>) -> Result<Json<TypedProperty>> {
    let property_repo = depot.repositories()?.properties_repo;
    let mut db_connection = depot.db_conn()?;
    let registration_id = depot.registration_id();
    let property_name = name.clone().to_string();

    match property_repo.get(
        &mut db_connection,
        property_name.clone(),
        Some(registration_id.clone()),
    ) {
        Ok(Some(property)) => Ok(Json(property)),
        Ok(None) => Err(ApiError::NotFoundError(format!(
            "Property {} not found",
            name
        ))),
        Err(error) => {
            error!(errorMsg=%error, name=property_name,registration_id=registration_id,"Error retrieving property from database");
            Err(ApiError::ServerError(error.to_string()))
        }
    }
}
