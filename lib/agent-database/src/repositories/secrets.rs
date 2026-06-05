use std::collections::HashMap;

use crate::errors::Result;
use crate::models::{Secret, SecretTag, Tags, UpdateSecret};
use crate::query::{DeleteQuery, FilterCondition, FilterOperator, SortCondition, SortDirection};
use crate::repositories::tags::{
    get_tag_ids, get_tag_names_for_filter_conditions, insert_or_get_tag,
};
use crate::schema::{secret_tags, secrets, tags};
use crate::traits::RepositoryByTags;
use crate::{
    DatabaseError, NewSecret, RepositoryDynamicQuery, RepositoryGenericInsert,
    RepositoryGenericUpdate,
};
use diesel::sql_types::Bool;
use diesel::sqlite::Sqlite;
use diesel::{debug_query, prelude::*};
use tracing::{debug, info, warn};

#[cfg(debug_assertions)]
use tracing::trace;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct SecretRepository;

impl SecretRepository {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

// Type alias for single-table conditions
type BoxedCondition = Box<dyn BoxableExpression<secrets::table, Sqlite, SqlType = Bool>>;

/// Build a single boxed condition from a filter
fn build_filter_condition(
    field: &str,
    operator: &FilterOperator,
    value: &str,
) -> Result<BoxedCondition> {
    let value_owned = value.to_string();
    let pattern = format!("%{}%", value_owned);
    match (field, operator) {
        // ID Filters
        ("id", FilterOperator::Eq) => Ok(Box::new(secrets::id.eq(value_owned.parse::<i32>()?))),
        ("id", FilterOperator::Gt) => Ok(Box::new(secrets::id.gt(value_owned.parse::<i32>()?))),
        ("id", FilterOperator::Gte) => Ok(Box::new(secrets::id.ge(value_owned.parse::<i32>()?))),
        ("id", FilterOperator::Lt) => Ok(Box::new(secrets::id.lt(value_owned.parse::<i32>()?))),
        ("id", FilterOperator::Lte) => Ok(Box::new(secrets::id.le(value_owned.parse::<i32>()?))),

        // Name filters
        ("name", FilterOperator::Eq) => Ok(Box::new(secrets::name.eq(value_owned.clone()))),
        ("name", FilterOperator::Like) => Ok(Box::new(secrets::name.like(pattern))),

        // Secret Type filters
        ("secret_type", FilterOperator::Eq) => {
            Ok(Box::new(secrets::secret_type.eq(value_owned.clone())))
        }
        ("secret_type", FilterOperator::Like) => Ok(Box::new(secrets::secret_type.like(pattern))),

        // Description Filters
        ("description", FilterOperator::Eq) => Ok(Box::new(
            secrets::description
                .eq(value_owned.clone())
                .assume_not_null(),
        )),
        ("description", FilterOperator::Like) => Ok(Box::new(
            secrets::description.like(pattern).assume_not_null(),
        )),

        // Value filters
        ("value", FilterOperator::Eq) => Ok(Box::new(secrets::value.eq(value_owned.clone()))),
        ("value", FilterOperator::Like) => Ok(Box::new(secrets::value.like(pattern))),

        // Source filters
        ("source", FilterOperator::Eq) => Ok(Box::new(secrets::source.eq(value_owned.clone()))),
        ("source", FilterOperator::Like) => Ok(Box::new(secrets::source.like(pattern))),

        // Encrytion Key ID filters
        ("encryption_key_id", FilterOperator::Eq) => Ok(Box::new(
            secrets::encryption_key_id.eq(value_owned.parse::<i32>()?),
        )),
        ("encryption_key_id", FilterOperator::Gt) => Ok(Box::new(
            secrets::encryption_key_id.gt(value_owned.parse::<i32>()?),
        )),
        ("encryption_key_id", FilterOperator::Gte) => Ok(Box::new(
            secrets::encryption_key_id.ge(value_owned.parse::<i32>()?),
        )),
        ("encryption_key_id", FilterOperator::Lt) => Ok(Box::new(
            secrets::encryption_key_id.lt(value_owned.parse::<i32>()?),
        )),
        ("encryption_key_id", FilterOperator::Lte) => Ok(Box::new(
            secrets::encryption_key_id.le(value_owned.parse::<i32>()?),
        )),

        // Created At / Updated At Filters
        ("created_at", FilterOperator::Eq) => {
            Ok(Box::new(secrets::created_at.eq(value_owned.clone())))
        }
        ("created_at", FilterOperator::Gt) => {
            Ok(Box::new(secrets::created_at.gt(value_owned.clone())))
        }
        ("created_at", FilterOperator::Lt) => {
            Ok(Box::new(secrets::created_at.lt(value_owned.clone())))
        }
        ("updated_at", FilterOperator::Eq) => Ok(Box::new(
            secrets::updated_at
                .eq(value_owned.clone())
                .assume_not_null(),
        )),
        ("updated_at", FilterOperator::Gt) => Ok(Box::new(
            secrets::updated_at
                .gt(value_owned.clone())
                .assume_not_null(),
        )),
        ("updated_at", FilterOperator::Lt) => Ok(Box::new(
            secrets::updated_at
                .lt(value_owned.clone())
                .assume_not_null(),
        )),

        _ => Err(DatabaseError::InvalidInput(format!(
            "Unsupported filter: {} with operator {:?}",
            field, operator
        ))),
    }
}

impl RepositoryDynamicQuery<Secret> for SecretRepository {
    /// Get secret records filtered by dynamic query parameters
    fn get_by_dynamic_query(
        &self,
        conn: &mut SqliteConnection,
        filters: &Vec<FilterCondition>,
        sort: Option<&Vec<SortCondition>>,
        page_size: i64,
        page: i64,
    ) -> Result<(Vec<Secret>, i64)> {
        debug!(
            filter_count = filters.len(),
            page, page_size, "Querying secrets"
        );

        // Separate tag filters from other filters
        let (tag_filters, secret_filters): (Vec<_>, Vec<_>) =
            filters.iter().partition(|f| f.field == "tags");

        // Build the base query
        let mut sql_query = secrets::table.select(Secret::as_select()).into_boxed();

        // Build the count query
        let mut count_query = secrets::table.select(Secret::as_select()).into_boxed();

        // Apply any tag filters - if present
        if let Ok(tag_names) = get_tag_names_for_filter_conditions(conn, tag_filters.clone()) {
            if let Ok(tag_secret_ids) = get_secret_ids_for_tags(conn, &tag_names) {
                if !tag_secret_ids.is_empty() {
                    sql_query = sql_query.filter(secrets::id.eq_any(tag_secret_ids.clone()));
                    count_query = count_query.filter(secrets::id.eq_any(tag_secret_ids));
                }
            }
        }

        // Apply filter conditions to both queries
        for filter in secret_filters {
            let condition = build_filter_condition(&filter.field, &filter.operator, &filter.value)?;
            sql_query = sql_query.filter(condition);

            // Build a fresh condition for count_query
            let count_condition =
                build_filter_condition(&filter.field, &filter.operator, &filter.value)?;
            count_query = count_query.filter(count_condition);
        }

        // Apply sorting
        if let Some(sort) = sort {
            for sort in sort {
                sql_query = match (sort.field.as_str(), &sort.direction) {
                    ("id", SortDirection::Asc) => sql_query.order_by(secrets::id.asc()),
                    ("id", SortDirection::Desc) => sql_query.order_by(secrets::id.desc()),
                    ("name", SortDirection::Asc) => sql_query.order_by(secrets::name.asc()),
                    ("name", SortDirection::Desc) => sql_query.order_by(secrets::name.desc()),
                    ("secret_type", SortDirection::Asc) => {
                        sql_query.order_by(secrets::secret_type.asc())
                    }
                    ("secret_type", SortDirection::Desc) => {
                        sql_query.order_by(secrets::secret_type.desc())
                    }
                    ("description", SortDirection::Asc) => {
                        sql_query.order_by(secrets::description.asc())
                    }
                    ("description", SortDirection::Desc) => {
                        sql_query.order_by(secrets::description.desc())
                    }
                    ("value", SortDirection::Asc) => sql_query.order_by(secrets::value.asc()),
                    ("value", SortDirection::Desc) => sql_query.order_by(secrets::value.desc()),
                    ("source", SortDirection::Asc) => sql_query.order_by(secrets::source.asc()),
                    ("source", SortDirection::Desc) => sql_query.order_by(secrets::source.desc()),
                    ("encryption_key_id", SortDirection::Asc) => {
                        sql_query.order_by(secrets::encryption_key_id.asc())
                    }
                    ("encryption_key_id", SortDirection::Desc) => {
                        sql_query.order_by(secrets::encryption_key_id.desc())
                    }
                    ("created_at", SortDirection::Asc) => {
                        sql_query.order_by(secrets::created_at.asc())
                    }
                    ("created_at", SortDirection::Desc) => {
                        sql_query.order_by(secrets::created_at.desc())
                    }
                    ("updated_at", SortDirection::Asc) => {
                        sql_query.order_by(secrets::updated_at.asc())
                    }
                    ("updated_at", SortDirection::Desc) => {
                        sql_query.order_by(secrets::updated_at.desc())
                    }
                    _ => sql_query,
                };
            }
        }

        #[cfg(debug_assertions)]
        trace!(query=%debug_query(&count_query),"Retrieving count of secret records from database");

        // count the filtered results
        let total_count: i64 = count_query.count().get_result(conn)?;

        // Apply pagination
        if page > 0 && page_size > 0 {
            sql_query = sql_query.limit(page_size).offset((page - 1) * page_size);
        }

        #[cfg(debug_assertions)]
        trace!(query=%debug_query(&sql_query),"Retrieving secret records from database");

        // Execute the query to get the results
        let secret_records = sql_query.load::<Secret>(conn)?;
        info!(
            total = total_count,
            returned = secret_records.len(),
            "Secrets query completed"
        );

        Ok((secret_records, total_count))
    }

    fn delete_by_dynamic_query(
        &self,
        conn: &mut SqliteConnection,
        query: &DeleteQuery,
    ) -> Result<usize> {
        debug!(
            filter_count = query.filters.len(),
            "Deleting secrets by query"
        );
        match self.get_by_dynamic_query(conn, &query.filters, None, 0, 0) {
            Ok((secret_records, num_records)) => {
                let record_count = num_records as usize;
                let result = conn
                    .transaction(|conn| {
                        for item in secret_records {
                            diesel::delete(secrets::table.filter(secrets::id.eq(item.id)))
                                .execute(conn)?;
                        }

                        Ok(record_count)
                    })
                    .map_err(|error: diesel::result::Error| DatabaseError::from(error));

                if result.is_ok() {
                    info!(deleted = record_count, "Secrets deleted");
                } else {
                    warn!(count = record_count, "Failed to delete secrets");
                }
                result
            }
            Err(error) => {
                warn!("Failed to query secrets for deletion");
                Err(error)
            }
        }
    }
}

impl RepositoryGenericInsert<Secret, NewSecret> for SecretRepository {
    fn create(&self, conn: &mut SqliteConnection, payload: NewSecret) -> Result<Secret> {
        debug!(name = %payload.name, "Creating secret");
        #[cfg(debug_assertions)]
        trace!(value = %payload.value, "Secret value details");

        let inserted = diesel::insert_into(secrets::table)
            .values(&payload)
            .returning(Secret::as_returning())
            .get_result(conn)?;
        info!(id = inserted.id, name = %inserted.name, "Secret created");
        Ok(inserted)
    }

    fn create_many(
        &self,
        conn: &mut SqliteConnection,
        payload: Vec<NewSecret>,
    ) -> Result<Vec<Secret>> {
        use diesel::Connection;

        debug!(count = payload.len(), "Creating multiple secrets");
        #[cfg(debug_assertions)]
        {
            for item in &payload {
                trace!(name = %item.name, value = %item.value, "Creating secret with details");
            }
        }

        let result = conn.transaction(|conn| {
            let mut secrets: Vec<Secret> = Vec::new();

            for item in payload {
                let inserted_secret_record = diesel::insert_into(secrets::table)
                    .values(&item)
                    .returning(Secret::as_returning())
                    .get_result(conn)?;

                secrets.push(inserted_secret_record);
            }

            Ok(secrets)
        });

        match &result {
            Ok(secrets) => info!(count = secrets.len(), "Secrets created"),
            Err(_) => warn!("Failed to create secrets"),
        }

        result.map_err(|error: diesel::result::Error| DatabaseError::from(error))
    }
}

impl RepositoryGenericUpdate<Secret, UpdateSecret> for SecretRepository {
    fn update(&self, conn: &mut SqliteConnection, payload: UpdateSecret) -> Result<Secret> {
        debug!(id = payload.id, "Updating secret");
        #[cfg(debug_assertions)]
        trace!(id = payload.id, value = %payload.value, "Secret update details");

        let updated = diesel::update(secrets::table)
            .set(&payload)
            .filter(secrets::id.eq(payload.id))
            .returning(Secret::as_returning())
            .get_result(conn)?;
        info!(id = updated.id, name = %updated.name, "Secret updated");
        Ok(updated)
    }
}

impl RepositoryByTags<Secret> for SecretRepository {
    fn get_tags_for(&self, conn: &mut SqliteConnection, entity: &Secret) -> Vec<Tags> {
        debug!(secret_id = entity.id, "Retrieving tags for secret");
        let tags = SecretTag::belonging_to(entity)
            .inner_join(tags::table)
            .select(Tags::as_select())
            .load(conn)
            .unwrap_or_else(|_| {
                warn!(secret_id = entity.id, "Failed to load tags for secret");
                vec![]
            });
        info!(
            secret_id = entity.id,
            count = tags.len(),
            "Tags retrieved for secret"
        );
        tags
    }

    fn get_tags_for_many(
        &self,
        conn: &mut SqliteConnection,
        entities: &[Secret],
    ) -> Result<HashMap<i32, Vec<Tags>>> {
        debug!(
            count = entities.len(),
            "Retrieving tags for multiple secrets"
        );
        let results = SecretTag::belonging_to(entities)
            .inner_join(tags::table)
            .select((secret_tags::secret_id, Tags::as_select()))
            .load::<(i32, Tags)>(conn)?;

        // Group by secret_id
        let mut tags_by_secret: HashMap<i32, Vec<Tags>> = HashMap::new();
        for (id, tag) in results {
            tags_by_secret.entry(id).or_insert_with(Vec::new).push(tag);
        }

        info!(
            secret_count = entities.len(),
            mapping_count = tags_by_secret.len(),
            "Tags retrieved for multiple secrets"
        );
        Ok(tags_by_secret)
    }

    fn create_tags_for(
        &self,
        conn: &mut SqliteConnection,
        entity: &Secret,
        tags: Vec<String>,
    ) -> Result<()> {
        debug!(
            secret_id = entity.id,
            tag_count = tags.len(),
            "Creating tags for secret"
        );
        let result = conn.transaction(|conn| {
            tags.into_iter().try_for_each(|tag| {
                let tag =
                    insert_or_get_tag(conn, &tag).map_err(|_e| diesel::result::Error::NotFound)?;

                let new_secret_tag = SecretTag {
                    secret_id: entity.id,
                    tag_id: tag.id,
                };

                diesel::insert_into(secret_tags::table)
                    .values(new_secret_tag)
                    .execute(conn)?;
                Ok(())
            })
        });

        match &result {
            Ok(_) => info!(secret_id = entity.id, "Tags created for secret"),
            Err(_) => warn!(secret_id = entity.id, "Failed to create tags for secret"),
        }

        result.map_err(|error: diesel::result::Error| DatabaseError::from(error))
    }

    fn delete_tags_for(&self, conn: &mut SqliteConnection, entity: &Secret) -> Result<usize> {
        debug!(secret_id = entity.id, "Deleting tags for secret");
        let deleted =
            diesel::delete(secret_tags::table.filter(secret_tags::secret_id.eq(entity.id)))
                .execute(conn)?;
        info!(
            secret_id = entity.id,
            deleted = deleted,
            "Tags deleted for secret"
        );
        Ok(deleted)
    }
}

fn get_secret_ids_for_tags(conn: &mut SqliteConnection, tags: &[String]) -> Result<Vec<i32>> {
    let tag_ids = get_tag_ids(conn, tags)?;

    let secret_ids = secret_tags::table
        .filter(secret_tags::tag_id.eq_any(&tag_ids))
        .select(secret_tags::secret_id)
        .distinct()
        .load::<i32>(conn)?;
    Ok(secret_ids)
}
