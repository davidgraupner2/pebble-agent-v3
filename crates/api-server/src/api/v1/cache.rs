use crate::api::extensions::DepotExt;
use crate::error::{ApiError, AppResult};
use crate::responses::PaginatedApiResponse;
use agent_database::query::{DeleteQuery, DynamicQuery, FilterCondition, FilterOperator};
use agent_database::traits::{RepositoryByTags, RepositoryDynamicQuery};
use agent_database::{
    Cache, CacheWithTags, CreateCacheRequest, NewCache, RepositoryGenericInsert,
    RepositoryGenericUpdate, Tags, UpdateCache, UpdateCacheRequest,
};
use chrono::{Duration, NaiveDateTime, Timelike, Utc};
use salvo::oapi::extract::JsonBody;
use salvo::prelude::*;
use salvo::Extractible;
use std::collections::HashMap;
use tracing::{debug, info, warn};

/// Build all V1 cache routes.
///
/// Mounted endpoints:
/// - `GET /cache_records`
/// - `POST /cache_records`
/// - `POST /cache_record`
/// - `PUT /cache_record`
/// - `DELETE /cache_records`
pub fn cache_router() -> Router {
    Router::new()
        .push(Router::with_path("cache_records").get(get_cache_records))
        .push(Router::with_path("cache_records").post(add_cache_records))
        .push(Router::with_path("cache_record").post(add_cache_record))
        .push(Router::with_path("cache_record").put(update_cache_record))
        .push(Router::with_path("cache_records").delete(delete_cache_records))
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

/// Retrieve cache records for the authenticated agent with filtering, sorting, and pagination.
///
/// Why this endpoint exists:
/// - Clients need list/search access to cache records without loading everything client-side.
/// - Query execution is scoped by `registration_id` from the bearer token.
///
/// Querystring guide:
/// - Pagination:
///   - `page=<number>` (default `1`)
///   - `page_size=<number>` (default `20`, max `100`)
/// - Sorting:
///   - `orderby=<field>` for ascending
///   - `orderby.desc=<field>` for descending
/// - Filters:
///   - `field=value` (implicit `eq`)
///   - `field.eq=value`, `field.neq=value`, `field.like=value`
///   - `field.gt=value`, `field.gte=value`, `field.lt=value`, `field.lte=value`
///   - `field.in=a,b,c`, `field.nin=a,b,c`
///
/// Allowed fields:
/// - `id`, `name`, `description`, `type`, `value`, `source`, `created_at`, `updated_at`, `expires_at`, `tags`
///
/// Response behavior:
/// - `200`: returns a paginated envelope; empty matches return `data: []` with pagination metadata.
/// - `400`: invalid query parameter format, unsupported operator/field, or pagination out of range.
/// - `401`: missing or invalid bearer token.
/// - `500`: repository or server failure.
#[endpoint(security(("bearer_token"=[])),tags("Cache Records"), status_codes(200, 400, 401, 500))]
async fn get_cache_records(
    req: &mut Request,
    depot: &mut Depot,
) -> AppResult<Json<PaginatedApiResponse<CacheWithTags>>> {
    let registration_id = depot.registration_id();

    // Extract the filter conditions passed in
    let query = match DynamicQuery::extract(req, depot).await {
        Ok(q) => q,
        Err(error) => {
            let mut message = format!("{:?}", error);

            // Some extractor errors are debug-printed as quoted strings.
            // Normalize that form so API clients see plain text without escapes.
            if message.starts_with('"') && message.ends_with('"') && message.len() >= 2 {
                message = message[1..message.len() - 1]
                    .replace("\\\"", "\"")
                    .replace("\\\\", "\\");
            }

            return Err(ApiError::BadRequest(message));
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
            registration_id,
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

/// Create a single cache record for the authenticated agent.
///
/// Querystring guide:
/// - This endpoint does not accept query parameters.
///
/// Response behavior:
/// - `200`: cache record created and returned with tags.
/// - `400`: validation error (for example duplicate name, invalid TTL/expires_at combination, or malformed body).
/// - `401`: missing or invalid bearer token.
/// - `500`: repository or server failure.
#[endpoint(security(("bearer_token"=[])),tags("Cache Records"), status_codes(200, 400, 401, 500))]
async fn add_cache_record(
    depot: &mut Depot,
    payload: JsonBody<CreateCacheRequest>,
) -> AppResult<Json<CacheWithTags>> {
    let cache_repo = depot.repositories()?.cache_repo;
    let mut db_connection = depot.db_conn()?;
    let registration_id = depot.registration_id();

    error_if_cache_already_exists(depot, payload.name.clone(), registration_id.clone())?;
    let proposed_cache_record = payload.into_inner();

    let new_cache_record = create_new_cache(proposed_cache_record.clone(), registration_id.clone());

    let cache_record = cache_repo
        .create(&mut db_connection, new_cache_record?)
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

/// Create multiple cache records for the authenticated agent in a single request.
///
/// Querystring guide:
/// - This endpoint does not accept query parameters.
///
/// Response behavior:
/// - `200`: cache records created and returned with tags.
/// - `400`: validation error (for example duplicate names, invalid TTL/expires_at combination, or malformed body).
/// - `401`: missing or invalid bearer token.
/// - `500`: repository or server failure.
#[endpoint(security(("bearer_token"=[])),tags("Cache Records"), status_codes(200, 400, 401, 500))]
async fn add_cache_records(
    depot: &mut Depot,
    payload: JsonBody<Vec<CreateCacheRequest>>,
) -> AppResult<Json<Vec<CacheWithTags>>> {
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
        new_cache_records.push(create_new_cache(cache_record, registration_id)?);
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

/// Update a single cache record for the authenticated agent.
///
/// Querystring guide:
/// - This endpoint does not accept query parameters.
///
/// Response behavior:
/// - `200`: cache record updated and returned with tags.
/// - `400`: validation error (for example invalid TTL/expires_at combination or malformed body).
/// - `401`: missing or invalid bearer token.
/// - `500`: repository or server failure.
#[endpoint(security(("bearer_token"=[])),tags("Cache Records"), status_codes(200, 400, 401, 500))]
async fn update_cache_record(
    depot: &mut Depot,
    payload: JsonBody<UpdateCacheRequest>,
) -> AppResult<Json<CacheWithTags>> {
    let cache_repo = depot.repositories()?.cache_repo;
    let mut db_connection = depot.db_conn()?;
    let registration_id = depot.registration_id();

    let proposed_cache_record = payload.into_inner();

    let updated_cache_record =
        create_update_cache(proposed_cache_record.clone(), registration_id.clone())?;

    let cache_record = cache_repo
        .update(&mut db_connection, updated_cache_record)
        .map_err(|e| ApiError::DataAccessError(e.to_string()))?;

    // Now Create the tags for the new cache record
    if let Some(tags) = proposed_cache_record.tags {
        cache_repo.create_tags_for(&mut db_connection, &cache_record, tags)?;
    }

    // Now get the tags for the cache record updated
    let tags = cache_repo.get_tags_for(&mut db_connection, &cache_record);

    // Build the response
    let response = build_cache_response(cache_record, tags);

    Ok(Json(response))
}

/// Delete cache records by dynamic filter for the authenticated agent.
///
/// Querystring guide:
/// - Filters use the same syntax/operators as `GET /cache_records`:
///   - `field=value`, `field.eq=value`, `field.neq=value`, `field.like=value`
///   - `field.gt=value`, `field.gte=value`, `field.lt=value`, `field.lte=value`
///   - `field.in=a,b,c`, `field.nin=a,b,c`
/// - Safety switch:
///   - `confirm_delete_all=true` is required when no filter is provided.
///
/// Allowed filter fields:
/// - `id`, `name`, `description`, `type`, `value`, `source`, `created_at`, `updated_at`, `expires_at`, `tags`
///
/// Response behavior:
/// - `200`: one or more records deleted.
/// - `400`: invalid query parameter format, unsupported field/operator, or missing `confirm_delete_all=true` for full delete.
/// - `401`: missing or invalid bearer token.
/// - `404`: query was valid but no matching records were found.
/// - `500`: repository or server failure.
#[endpoint(security(("bearer_token"=[])),tags("Cache Records"), status_codes(200, 400, 401, 404, 500))]
async fn delete_cache_records(depot: &mut Depot, req: &mut Request) -> AppResult<String> {
    let registration_id = depot.registration_id();

    // Extract the filter conditions passed in
    let query = match DeleteQuery::extract(req, depot).await {
        Ok(q) => q,
        Err(error) => {
            let mut message = format!("{:?}", error);

            // Some extractor errors are debug-printed as quoted strings.
            // Normalize that form so API clients see plain text without escapes.
            if message.starts_with('"') && message.ends_with('"') && message.len() >= 2 {
                message = message[1..message.len() - 1]
                    .replace("\\\"", "\"")
                    .replace("\\\\", "\\");
            }

            return Err(ApiError::BadRequest(message));
        }
    };

    debug!(
        filters = query.filters.len(),
        confirm_delete_all = query.confirm_delete_all,
        "Deleting cache records"
    );

    // Validate fields before running Diesel queries
    if let Err(err) = query.validate_fields(&CACHE_ALLOWED_FIELDS) {
        let allowed_fields = CACHE_ALLOWED_FIELDS.join(", ");
        return Err(ApiError::BadRequest(format!(
            "Invalid cache query: {}. Allowed fields include: {}",
            err, allowed_fields
        )));
    }

    if query.filters.is_empty() && !query.confirm_delete_all {
        return Err(ApiError::BadRequest(
            "Cannot delete all records without explicit confirmation. Use ?confirm_delete_all=true"
                .to_string(),
        ));
    }

    let cache_repo = depot.repositories()?.cache_repo;
    let mut db_connection = depot.db_conn()?;

    let num_records =
        cache_repo.delete_by_dynamic_query(&mut db_connection, &query, registration_id)?;

    if num_records == 0 {
        return Err(ApiError::NotFoundError(
            "No matching records found to delete!".to_string(),
        ));
    } else {
        // let response = ApiResponse::ok(format!("{} cache record(s) deleted", num_records));
        Ok(format!("{} cache record(s) deleted", num_records))
    }
}

/////////////////////////////////
// Helper Functions - start here
/////////////////////////////////

fn error_if_cache_already_exists(
    depot: &mut Depot,
    cache_name: impl Into<String> + Clone,
    registration_id: impl Into<String> + Clone,
) -> AppResult<()> {
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
        .get_by_dynamic_query(
            &mut db_connection,
            &filters,
            None,
            0,
            0,
            registration_id.into(),
        )
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

fn create_new_cache(cache: CreateCacheRequest, registration_id: String) -> AppResult<NewCache> {
    let expires_at = compute_expires_at(cache.expires_at, cache.ttl_seconds)?;

    Ok(NewCache {
        registration_id,
        name: cache.name,
        description: cache.description,
        type_: cache.type_,
        value: cache.value,
        source: cache.source.unwrap_or_default(),
        expires_at,
    })
}

fn create_update_cache(
    cache: UpdateCacheRequest,
    registration_id: String,
) -> AppResult<UpdateCache> {
    let expires_at = compute_expires_at(cache.expires_at, cache.ttl_seconds)?;

    Ok(UpdateCache {
        id: cache.id,
        registration_id,
        name: cache.name,
        description: cache.description,
        type_: cache.type_,
        value: cache.value,
        source: cache.source,
        expires_at,
    })
}

fn compute_expires_at(
    explicit_expires_at: Option<NaiveDateTime>,
    ttl_seconds: Option<i64>,
) -> AppResult<Option<NaiveDateTime>> {
    match (explicit_expires_at, ttl_seconds) {
        (Some(ts), None) => Ok(Some(ts)),
        (None, Some(ttl)) if ttl > 0 => {
            let now = Utc::now().naive_utc();
            let now = now.with_nanosecond(0).unwrap();
            Ok(Some(now + Duration::seconds(ttl)))
        }
        (None, None) => Ok(None), // never expires
        (Some(_), Some(_)) => Err(ApiError::BadRequest(
            "Provide either expires_at or ttl_seconds, not both".to_string(),
        )),
        (None, Some(_)) => Err(ApiError::BadRequest("ttl_seconds must be > 0".to_string())),
    }
}
