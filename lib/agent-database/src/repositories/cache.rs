use std::collections::HashMap;

use crate::RepositoryDynamicQuery;
use crate::errors::Result;
use crate::models::{Cache, CacheTag, NewCache, Tags, UpdateCache};
use crate::query::{DeleteQuery, FilterCondition, FilterOperator, SortCondition, SortDirection};
use crate::repositories::tags::{
    get_tag_ids, get_tag_names_for_filter_conditions, insert_or_get_tag,
};
use crate::schema::{cache, cache_tags, tags};
use crate::traits::RepositoryByTags;
use crate::{DatabaseError, RepositoryGenericUpdate, traits::RepositoryGenericInsert};
use diesel::sql_types::Bool;
use diesel::sqlite::Sqlite;
use diesel::{debug_query, prelude::*};

#[cfg(debug_assertions)]
use tracing::trace;

use tracing::{debug, info, warn};

/// Repository for cache-related database operations.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct CacheRepository {}

impl CacheRepository {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

// Need this type for common condition expressions
type BoxedCondition = Box<dyn BoxableExpression<cache::table, Sqlite, SqlType = Bool>>;

/// Build a single boxed condition from a filter
fn build_filter_condition(
    field: &str,
    operator: &FilterOperator,
    value: &str,
) -> Result<BoxedCondition> {
    let value_owned = value.to_string();
    match (field, operator) {
        // Non-nullable columns (these are fine as-is)
        ("id", FilterOperator::Eq) => Ok(Box::new(cache::id.eq(value_owned.parse::<i32>()?))),
        ("id", FilterOperator::Gt) => Ok(Box::new(cache::id.gt(value_owned.parse::<i32>()?))),
        ("id", FilterOperator::Gte) => Ok(Box::new(cache::id.ge(value_owned.parse::<i32>()?))),
        ("id", FilterOperator::Lt) => Ok(Box::new(cache::id.lt(value_owned.parse::<i32>()?))),
        ("id", FilterOperator::Lte) => Ok(Box::new(cache::id.le(value_owned.parse::<i32>()?))),

        // Name filters
        ("name", FilterOperator::Eq) => Ok(Box::new(cache::name.eq(value_owned.clone()))),
        ("name", FilterOperator::Like) => {
            let pattern = format!("%{}%", value_owned);
            Ok(Box::new(cache::name.like(pattern)))
        }

        // Nullable description column
        ("description", FilterOperator::Eq) => Ok(Box::new(
            cache::description.eq(value_owned.clone()).assume_not_null(),
        )),
        ("description", FilterOperator::Like) => {
            let pattern = format!("%{}%", value_owned);
            Ok(Box::new(cache::description.like(pattern).assume_not_null()))
        }

        // Type filters
        ("type", FilterOperator::Eq) => Ok(Box::new(cache::type_.eq(value_owned.clone()))),
        ("type", FilterOperator::Like) => {
            let pattern = format!("%{}%", value_owned);
            Ok(Box::new(cache::type_.like(pattern)))
        }

        // Value filters
        ("value", FilterOperator::Eq) => Ok(Box::new(cache::value.eq(value_owned.clone()))),
        ("value", FilterOperator::Like) => {
            let pattern = format!("%{}%", value_owned);
            Ok(Box::new(cache::value.like(pattern)))
        }

        // Source filters
        ("source", FilterOperator::Eq) => Ok(Box::new(cache::source.eq(value_owned.clone()))),
        ("source", FilterOperator::Like) => {
            let pattern = format!("%{}%", value_owned);
            Ok(Box::new(cache::source.like(pattern)))
        }

        ("created_at", FilterOperator::Eq) => {
            Ok(Box::new(cache::created_at.eq(value_owned.clone())))
        }
        ("created_at", FilterOperator::Gt) => {
            Ok(Box::new(cache::created_at.gt(value_owned.clone())))
        }
        ("created_at", FilterOperator::Lt) => {
            Ok(Box::new(cache::created_at.lt(value_owned.clone())))
        }
        ("updated_at", FilterOperator::Eq) => Ok(Box::new(
            cache::updated_at.eq(value_owned.clone()).assume_not_null(),
        )),
        ("updated_at", FilterOperator::Gt) => Ok(Box::new(
            cache::updated_at.gt(value_owned.clone()).assume_not_null(),
        )),
        ("updated_at", FilterOperator::Lt) => Ok(Box::new(
            cache::updated_at.lt(value_owned.clone()).assume_not_null(),
        )),
        ("expires_at", FilterOperator::Eq) => Ok(Box::new(
            cache::expires_at.eq(value_owned.clone()).assume_not_null(),
        )),
        ("expires_at", FilterOperator::Gt) => Ok(Box::new(
            cache::expires_at.gt(value_owned.clone()).assume_not_null(),
        )),
        ("expires_at", FilterOperator::Lt) => Ok(Box::new(
            cache::expires_at.lt(value_owned.clone()).assume_not_null(),
        )),

        _ => Err(DatabaseError::InvalidInput(format!(
            "Unsupported filter: {} with operator {:?}",
            field, operator
        ))),
    }
}

impl RepositoryDynamicQuery<Cache> for CacheRepository {
    /// Get cache records filtered by dynamic query parameters
    fn get_by_dynamic_query(
        &self,
        conn: &mut SqliteConnection,
        filters: &Vec<FilterCondition>,
        sort: Option<&Vec<SortCondition>>,
        page_size: i64,
        page: i64,
    ) -> Result<(Vec<Cache>, i64)> {
        debug!(
            page,
            page_size,
            ?filters,
            ?sort,
            "Retrieving page of Cache records"
        );

        // Separate tag filters from other filters
        let (tag_filters, cache_filters): (Vec<_>, Vec<_>) =
            filters.iter().partition(|f| f.field == "tags");

        // Build the base query
        let mut sql_query = cache::table.select(Cache::as_select()).into_boxed();

        // Build the count query
        let mut count_query = cache::table.select(Cache::as_select()).into_boxed();

        // Apply any tag filters - if present
        if let Ok(tag_names) = get_tag_names_for_filter_conditions(conn, tag_filters.clone()) {
            if let Ok(tag_secret_ids) = get_cache_ids_for_tags(conn, &tag_names) {
                if !tag_secret_ids.is_empty() {
                    sql_query = sql_query.filter(cache::id.eq_any(tag_secret_ids.clone()));
                    count_query = count_query.filter(cache::id.eq_any(tag_secret_ids));
                }
            }
        }

        // Apply filter conditions to both queries
        for filter in cache_filters {
            let condition = build_filter_condition(&filter.field, &filter.operator, &filter.value)?;
            sql_query = sql_query.filter(condition);

            let count_condition =
                build_filter_condition(&filter.field, &filter.operator, &filter.value)?;
            count_query = count_query.filter(count_condition);
        }

        // Apply sorting
        if let Some(sort) = sort {
            for sort in sort {
                sql_query = match (sort.field.as_str(), &sort.direction) {
                    ("id", SortDirection::Asc) => sql_query.order_by(cache::id.asc()),
                    ("id", SortDirection::Desc) => sql_query.order_by(cache::id.desc()),
                    ("name", SortDirection::Asc) => sql_query.order_by(cache::name.asc()),
                    ("name", SortDirection::Desc) => sql_query.order_by(cache::name.desc()),
                    ("description", SortDirection::Asc) => {
                        sql_query.order_by(cache::description.asc())
                    }
                    ("description", SortDirection::Desc) => {
                        sql_query.order_by(cache::description.desc())
                    }
                    ("type", SortDirection::Asc) => sql_query.order_by(cache::type_.asc()),
                    ("type", SortDirection::Desc) => sql_query.order_by(cache::type_.desc()),
                    ("value", SortDirection::Asc) => sql_query.order_by(cache::value.asc()),
                    ("value", SortDirection::Desc) => sql_query.order_by(cache::value.desc()),
                    ("source", SortDirection::Asc) => sql_query.order_by(cache::source.asc()),
                    ("source", SortDirection::Desc) => sql_query.order_by(cache::source.desc()),
                    ("created_at", SortDirection::Asc) => {
                        sql_query.order_by(cache::created_at.asc())
                    }
                    ("created_at", SortDirection::Desc) => {
                        sql_query.order_by(cache::created_at.desc())
                    }
                    ("updated_at", SortDirection::Asc) => {
                        sql_query.order_by(cache::updated_at.asc())
                    }
                    ("updated_at", SortDirection::Desc) => {
                        sql_query.order_by(cache::updated_at.desc())
                    }
                    ("expires_at", SortDirection::Asc) => {
                        sql_query.order_by(cache::expires_at.asc())
                    }
                    ("expires_at", SortDirection::Desc) => {
                        sql_query.order_by(cache::expires_at.desc())
                    }
                    _ => sql_query,
                };
            }
        }

        #[cfg(debug_assertions)]
        trace!(query=%debug_query(&count_query),"Retrieving count of cache records from database");

        // count the filtered results
        let total_count: i64 = count_query.count().get_result(conn)?;

        // Apply pagination
        if page > 0 && page_size > 0 {
            sql_query = sql_query.limit(page_size).offset((page - 1) * page_size);
        }

        #[cfg(debug_assertions)]
        trace!(query=%debug_query(&sql_query),"Retrieving cache records from database");

        // Execute
        let cache_records = sql_query.load::<Cache>(conn)?;

        Ok((cache_records, total_count))
    }

    fn delete_by_dynamic_query(
        &self,
        conn: &mut SqliteConnection,
        query: &DeleteQuery,
    ) -> Result<usize> {
        match self.get_by_dynamic_query(conn, &query.filters, None, 0, 0) {
            Ok((cache_records, num_records)) => conn
                .transaction(|conn| {
                    for item in cache_records {
                        diesel::delete(cache::table.filter(cache::id.eq(item.id))).execute(conn)?;
                    }

                    Ok(num_records as usize)
                })
                .map_err(|error| error),
            Err(error) => Err(error),
        }
    }
}

impl RepositoryGenericInsert<Cache, NewCache> for CacheRepository {
    fn create(&self, conn: &mut SqliteConnection, payload: NewCache) -> Result<Cache> {
        debug!(name = %payload.name, type_ = %payload.type_, "Creating cache record");

        #[cfg(debug_assertions)]
        trace!(?payload, "Cache payload details");

        let result = diesel::insert_into(cache::table)
            .values(&payload)
            .returning(Cache::as_returning())
            .get_result(conn)
            .map_err(|e| {
                warn!(name = %payload.name, error = %e, "Failed to create cache record");
                e.into()
            });

        if let Ok(ref cache) = result {
            info!(id = cache.id, name = %cache.name, type_ = %cache.type_, "Cache record created");
        }
        result
    }

    /// Creates multiple cache entries with optional tags, in a transaction.
    fn create_many(
        &self,
        conn: &mut SqliteConnection,
        payload: Vec<NewCache>,
    ) -> Result<Vec<Cache>> {
        use diesel::Connection;

        let count = payload.len();
        debug!(count, "Creating batch of cache records");

        #[cfg(debug_assertions)]
        trace!(?payload, "Batch details");

        let result = conn.transaction(|conn| {
            let mut cache_records: Vec<Cache> = Vec::new();

            for (idx, item) in payload.into_iter().enumerate() {
                #[cfg(debug_assertions)]
                trace!(batch_index = idx, name = %item.name, "Inserting cache record");

                let inserted_cache_record = diesel::insert_into(cache::table)
                    .values(&item)
                    .returning(Cache::as_returning())
                    .get_result(conn)
                    .map_err(|e| {
                        warn!(batch_index = idx, error = %e, "Failed to insert cache record in batch");
                        e
                    })?;

                cache_records.push(inserted_cache_record);
            }

            Ok(cache_records)
        }).map_err(|error| {
            warn!(count, error = %error, "Batch cache creation failed");
            error
        });

        if let Ok(ref records) = result {
            info!(count = records.len(), "Successfully created cache batch");
        }
        result
    }
}

impl RepositoryGenericUpdate<Cache, UpdateCache> for CacheRepository {
    fn update(&self, conn: &mut SqliteConnection, payload: UpdateCache) -> Result<Cache> {
        debug!(id = payload.id, "Updating cache record");

        #[cfg(debug_assertions)]
        trace!(?payload, "Update payload");

        let result = diesel::update(cache::table)
            .set(&payload)
            .filter(cache::id.eq(payload.id))
            .returning(Cache::as_returning())
            .get_result(conn)
            .map_err(|e| {
                warn!(id = payload.id, error = %e, "Failed to update cache record");
                e.into()
            });

        if let Ok(ref cache) = result {
            info!(id = cache.id, name = %cache.name, "Cache record updated");
        }
        result
    }
}

impl RepositoryByTags<Cache> for CacheRepository {
    fn get_tags_for(&self, conn: &mut SqliteConnection, entity: &Cache) -> Vec<Tags> {
        debug!(cache_id = entity.id, cache_name = %entity.name, "Retrieving tags for cache");

        let tags = CacheTag::belonging_to(entity)
            .inner_join(tags::table)
            .select(Tags::as_select())
            .load(conn)
            .unwrap_or_else(|e| {
                warn!(cache_id = entity.id, error = %e, "Failed to load tags for cache");
                vec![]
            });

        #[cfg(debug_assertions)]
        trace!(
            cache_id = entity.id,
            tag_count = tags.len(),
            "Retrieved tags"
        );
        tags
    }

    fn get_tags_for_many(
        &self,
        conn: &mut SqliteConnection,
        entities: &[Cache],
    ) -> Result<HashMap<i32, Vec<Tags>>> {
        debug!(
            cache_count = entities.len(),
            "Retrieving tags for multiple caches"
        );

        let results = CacheTag::belonging_to(entities)
            .inner_join(tags::table)
            .select((cache_tags::cache_id, Tags::as_select()))
            .load::<(i32, Tags)>(conn)
            .map_err(|e| {
                warn!(cache_count = entities.len(), error = %e, "Failed to load tags for multiple caches");
                DatabaseError::from(e)
            })?;

        // Group by cache_id
        let mut tags_by_cache: HashMap<i32, Vec<Tags>> = HashMap::new();
        for (id, tag) in results {
            tags_by_cache.entry(id).or_insert_with(Vec::new).push(tag);
        }

        info!(
            cache_count = entities.len(),
            total_unique_caches = tags_by_cache.len(),
            "Tags retrieved for batch"
        );

        #[cfg(debug_assertions)]
        trace!(?tags_by_cache, "Tag mapping");

        Ok(tags_by_cache)
    }

    fn create_tags_for(
        &self,
        conn: &mut SqliteConnection,
        entity: &Cache,
        tags: Vec<String>,
    ) -> Result<()> {
        let tag_count = tags.len();
        debug!(cache_id = entity.id, cache_name = %entity.name, tag_count, "Creating tags for cache");

        #[cfg(debug_assertions)]
        trace!(?tags, "Tags to create");

        conn.transaction(|conn| {
            tags.into_iter().try_for_each(|tag| {
                #[cfg(debug_assertions)]
                trace!(cache_id = entity.id, tag_name = %tag, "Processing tag");
                                let tag = insert_or_get_tag(conn, &tag)
                    .map_err(|e| {
                        warn!(cache_id = entity.id, tag_name = %tag, error = %e, "Failed to insert/get tag");
                        diesel::result::Error::NotFound
                    })?;

                let new_cache_tag = CacheTag {
                    cache_id: entity.id,
                    tag_id: tag.id,
                };

                diesel::insert_into(cache_tags::table)
                    .values(new_cache_tag)
                    .execute(conn)
                    .map(|_| ())
                    .map_err(|e| {
                        warn!(cache_id = entity.id, tag_id = tag.id, error = %e, "Failed to link tag to cache");
                        e
                    })
            })
        }).map_err(|error: diesel::result::Error| {
            warn!(cache_id = entity.id, tag_count, error = %error, "Failed to create tags for cache");
            DatabaseError::from(error)
        })?;

        info!(
            cache_id = entity.id,
            tag_count, "Successfully created tags for cache"
        );
        Ok(())
    }

    fn delete_tags_for(&self, conn: &mut SqliteConnection, entity: &Cache) -> Result<usize> {
        debug!(cache_id = entity.id, cache_name = %entity.name, "Deleting all tags for cache");

        let count = diesel::delete(cache_tags::table.filter(cache_tags::cache_id.eq(entity.id)))
            .execute(conn)?;

        info!(
            cache_id = entity.id,
            deleted_count = count,
            "Tags deleted for cache"
        );
        Ok(count)
    }
}

fn get_cache_ids_for_tags(conn: &mut SqliteConnection, tags: &[String]) -> Result<Vec<i32>> {
    let tag_ids = get_tag_ids(conn, tags)?;

    let cache_ids = cache_tags::table
        .filter(cache_tags::tag_id.eq_any(&tag_ids))
        .select(cache_tags::cache_id)
        .distinct()
        .load::<i32>(conn)?;
    Ok(cache_ids)
}
