use std::collections::HashMap;

use crate::api::extensions::DepotExt;
use crate::error::{ApiError, Result};
use crate::responses::PaginatedApiResponse;
use agent_database::query::{DynamicQuery, FilterCondition, FilterOperator};
use agent_database::traits::{RepositoryByTags, RepositoryDynamicQuery};
use agent_database::{
    Cache, CacheWithTags, CreateCacheRequest, NewCache, RepositoryGenericInsert,
    RepositoryGenericUpdate, Tags, UpdateCache, UpdateCacheRequest,
};
use diesel::{
    r2d2::{ConnectionManager, PooledConnection},
    SqliteConnection,
};
use salvo::oapi::extract::JsonBody;
use salvo::oapi::extract::PathParam;
use salvo::prelude::*;
use salvo::Extractible;
use serde::Deserialize;
use serde_json::Value;
use tracing::{debug, error, info, warn};

/// Build all V1 cache routes.
///
/// Mounted endpoints:
/// - `POST /registration/challenge`
/// - `POST /registration/complete`
pub fn cache_router() -> Router {
    Router::new()
        .push(Router::with_path("cache_records").get(get_cache_records))
        .push(Router::with_path("cache_records").post(add_cache_records))
        .push(Router::with_path("cache_record").post(add_cache_record))
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
async fn get_cache_records(
    req: &mut Request,
    depot: &mut Depot,
) -> Result<Json<PaginatedApiResponse<CacheWithTags>>> {
    // Extract the filter conditions passed in
    let query = match DynamicQuery::extract(req, depot).await {
        Ok(q) => q,
        Err(error) => {
            return Err(ApiError::BadRequest(format!(
                "Validation error: {:#?}",
                error
            )))
        }
    };

    debug!(
        filters = query.filters.len(),
        page = query.page,
        page_size = query.page_size,
        "Getting cache records"
    );

    // Validate fields before running Diesel queries
    if let Err(err) = query.validate_fields(&CACHE_ALLOWED_FIELDS) {
        let allowed_fields = CACHE_ALLOWED_FIELDS.join(", ");
        return Err(ApiError::BadRequest(format!(
            "Invalid cache query: {}. Allowed fields include: {}",
            err, allowed_fields
        )));
    }

    let cache_repo = depot.repositories()?.cache_repo;
    let mut db_connection = depot.db_conn()?;

    // Get All the matching cache records
    let (cache_records, total) = cache_repo
        .get_by_dynamic_query(
            &mut db_connection,
            &query.filters,
            Some(&query.sort),
            query.page_size,
            query.page,
        )
        .map_err(|e| {
            warn!("Failed to query cache records: {}", e);
            ApiError::from(e)
        })?;

    // Now get the tags for the cache_records found
    let tag_hashmap = cache_repo.get_tags_for_many(&mut db_connection, &cache_records)?;

    // // Apply the tags to the cache records
    let cache_with_tags: Vec<CacheWithTags> = cache_records
        .into_iter()
        .map(|cache| CacheWithTags {
            cache: cache.clone(),
            tags: tag_hashmap.get(&cache.id).cloned().unwrap_or_default(),
        })
        .collect();

    info!(
        returned = cache_with_tags.len(),
        total = total,
        "Cache records retrieved"
    );

    let response = PaginatedApiResponse::ok(cache_with_tags, total, query.page, query.page_size);

    Ok(Json(response))
}

#[endpoint(security(("bearer_token"=[])),tags("Cache Records"), status_codes(200, 400, 401, 500))]
async fn add_cache_record(
    depot: &mut Depot,
    payload: JsonBody<CreateCacheRequest>,
) -> Result<Json<CacheWithTags>> {
    let cache_repo = depot.repositories()?.cache_repo;
    let mut db_connection = depot.db_conn()?;
    let registration_id = depot.registration_id();

    error_if_cache_already_exists(depot, payload.name.clone(), registration_id.clone())?;
    let proposed_cache_record = payload.into_inner();

    let new_cache_record = create_new_cache(proposed_cache_record.clone(), registration_id.clone());

    let cache_record = cache_repo
        .create(&mut db_connection, new_cache_record)
        .map_err(|e| ApiError::DataAccessError(e.to_string()))?;

    // Now Create the tags for the new cache record
    if let Some(tags) = proposed_cache_record.tags {
        cache_repo.create_tags_for(&mut db_connection, &cache_record, tags)?;
    }

    // Now get the tags for the cache record created
    let tags = cache_repo.get_tags_for(&mut db_connection, &cache_record);

    // Build the response
    let response = build_cache_response(cache_record, tags);

    Ok(Json(response))
}

#[endpoint(security(("bearer_token"=[])),tags("Cache Records"), status_codes(200, 400, 401, 500))]
async fn add_cache_records(
    depot: &mut Depot,
    payload: JsonBody<Vec<CreateCacheRequest>>,
) -> Result<Json<Vec<CacheWithTags>>> {
    let cache_repo = depot.repositories()?.cache_repo;
    let mut db_connection = depot.db_conn()?;
    let proposed_cache_records = payload.into_inner();

    // Create the hashmap before the first loop
    let mut tags_map: HashMap<String, Vec<String>> = HashMap::new();

    // Create a Vector to store the cache records to create
    let mut new_cache_records = vec![];

    //Create a vector to store the cache records to be returned
    let mut returned_cache_records: Vec<CacheWithTags> = vec![];

    for cache_record in proposed_cache_records {
        let registration_id = depot.registration_id();

        // Check for duplicate name
        error_if_cache_already_exists(depot, cache_record.name.clone(), registration_id.clone())?;

        // Extract tags before moving cache_record
        if let Some(tags) = cache_record.tags.clone() {
            tags_map.insert(cache_record.name.clone(), tags);
        }

        // Collect all the new cache records
        new_cache_records.push(create_new_cache(cache_record, registration_id));
    }

    // Create all cache records
    let added_cache_records = cache_repo
        .create_many(&mut db_connection, new_cache_records)
        .map_err(|e| ApiError::DataAccessError(e.to_string()))?;

    // Now loop through all the new cache records and create the tags for each
    for added_cache_record in added_cache_records {
        // Create any tags associated with the cache record
        let tags_to_be_created = tags_map
            .get(&added_cache_record.name)
            .cloned()
            .unwrap_or_default();
        cache_repo.create_tags_for(&mut db_connection, &added_cache_record, tags_to_be_created)?;

        // Get the tags created for this cache records
        let tags = cache_repo.get_tags_for(&mut db_connection, &added_cache_record);

        // Add to the cache records to return
        returned_cache_records.push(build_cache_response(added_cache_record, tags));
    }

    Ok(Json(returned_cache_records))
}

/////////////////////////////////
// Helper Functions - start here
/////////////////////////////////

fn error_if_cache_already_exists(
    depot: &mut Depot,
    cache_name: impl Into<String> + Clone,
    registration_id: impl Into<String> + Clone,
) -> Result<()> {
    let cache_repo = depot.repositories()?.cache_repo;
    let mut db_connection = depot.db_conn()?;

    // Build filter for checking if secret already exists
    let filters = vec![
        FilterCondition {
            field: "name".to_string(),
            operator: FilterOperator::Eq,
            value: cache_name.clone().into(),
        },
        FilterCondition {
            field: "registration_id".to_string(),
            operator: FilterOperator::Eq,
            value: registration_id.clone().into(),
        },
    ];

    //
    let existing_cache_record = cache_repo
        .get_by_dynamic_query(&mut db_connection, &filters, None, 0, 0)
        .map_err(|e| ApiError::DataAccessError(e.to_string()))?
        .0
        .into_iter()
        .next();

    if existing_cache_record.is_some() {
        return Err(ApiError::DuplicateNameError(
            "cache record".to_string(),
            cache_name.into(),
        ));
    }

    Ok(())
}

fn build_cache_response(cache: Cache, tags: Vec<Tags>) -> CacheWithTags {
    CacheWithTags {
        cache: Cache { ..cache },
        tags,
    }
}

fn create_new_cache(cache: CreateCacheRequest, registration_id: String) -> NewCache {
    NewCache {
        registration_id,
        name: cache.name,
        description: cache.description,
        type_: cache.type_,
        value: cache.value,
        source: cache.source.unwrap_or_default(),
        expires_at: cache.expires_at,
    }
}

fn create_update_cache(cache: UpdateCacheRequest, registration_id: String) -> UpdateCache {
    UpdateCache {
        id: cache.id,
        registration_id,
        name: cache.name,
        description: cache.description,
        type_: cache.type_,
        value: cache.value,
        source: cache.source,
        expires_at: cache.expires_at,
    }
}
