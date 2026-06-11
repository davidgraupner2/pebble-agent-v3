use std::convert::Infallible;

use crate::api::extensions::DepotExt;
use crate::error::{ApiError, Result};
use agent_database::query::DynamicQuery;
use agent_database::traits::{RepositoryByTags, RepositoryDynamicQuery};
use agent_database::{Cache, CacheWithTags, CreateCacheRequest, NewCache, RepositoryGenericUpdate};
use salvo::oapi::extract::JsonBody;
use salvo::oapi::extract::PathParam;
use salvo::prelude::*;
use salvo::Extractible;
use serde::Deserialize;
use serde_json::Value;
use tracing::error;

/// Build all V1 cache routes.
///
/// Mounted endpoints:
/// - `POST /registration/challenge`
/// - `POST /registration/complete`
pub fn cache_router() -> Router {
    Router::new().push(Router::with_path("cache").get(get_cache_records))
    //         .push(Router::with_path("registration/complete").post(complete_registration_challenge))
}

// ============= Field Whitelist =============
// Define allowed fields for cache table
const CACHE_ALLOWED_FIELDS: &[&str] = &[
    "id",
    "name",
    "description",
    "type",
    "value",
    "source",
    "created_at",
    "updated_at",
    "expires_at",
    "tags",
];

#[endpoint(security(("bearer_token"=[])),tags("Cache Records"), status_codes(200, 401, 404, 500))]
async fn get_cache_records(req: &mut Request, res: &mut Response, depot: &mut Depot) {
    // Extract the filter conditions passed in
    let query = match DynamicQuery::extract(req, depot).await {
        Ok(q) => q,
        Err(status_err) => {
            res.render(StatusCode::BAD_REQUEST);
            res.render(format!("Validation error: {:#?}", status_err));
            return;
        }
    };

    // Validate fields before running Diesel queries
    if let Err(err) = query.validate_fields(&CACHE_ALLOWED_FIELDS) {
        res.status_code(StatusCode::BAD_REQUEST);
        res.render(format!(
            "Invalid cache field in query '{}'. Valid fields include '{:#?}'",
            err, CACHE_ALLOWED_FIELDS
        ));
        return;
    }

    // Process your Diesel logic here using query.filters, query.sort, etc.
    res.render(format!(
        "Successfully extracted query structure: {:?}",
        query
    ));
}
