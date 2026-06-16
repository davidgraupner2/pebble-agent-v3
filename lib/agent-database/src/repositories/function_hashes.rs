use crate::errors::Result;
use crate::models::{FunctionHash, NewFunctionHash};
use crate::query::{DeleteQuery, FilterCondition, FilterOperator, SortCondition, SortDirection};
use crate::schema::function_hashes::{self};
use crate::traits::RepositoryGenericInsert;
use crate::{DatabaseError, RepositoryDynamicQuery, RepositoryGenericUpdate, UpdateFunctionHash};
use diesel::sql_types::Bool;
use diesel::sqlite::Sqlite;
use diesel::{debug_query, prelude::*};
use tracing::{debug, info, warn};

#[cfg(debug_assertions)]
use tracing::trace;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct FunctionHashRepository;

impl FunctionHashRepository {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

// Need this type for common condition expressions
type BoxedCondition = Box<dyn BoxableExpression<function_hashes::table, Sqlite, SqlType = Bool>>;

/// Build a single boxed condition from a filter
fn build_filter_condition(
    field: &str,
    operator: &FilterOperator,
    value: &str,
) -> Result<BoxedCondition> {
    let value_owned = value.to_string();
    match (field, operator) {
        // ID Filters
        ("id", FilterOperator::Eq) => Ok(Box::new(
            function_hashes::id.eq(value_owned.parse::<i32>()?),
        )),
        ("id", FilterOperator::Gt) => Ok(Box::new(
            function_hashes::id.gt(value_owned.parse::<i32>()?),
        )),
        ("id", FilterOperator::Gte) => Ok(Box::new(
            function_hashes::id.ge(value_owned.parse::<i32>()?),
        )),
        ("id", FilterOperator::Lt) => Ok(Box::new(
            function_hashes::id.lt(value_owned.parse::<i32>()?),
        )),
        ("id", FilterOperator::Lte) => Ok(Box::new(
            function_hashes::id.le(value_owned.parse::<i32>()?),
        )),

        //Function Hash filters
        ("function_hash", FilterOperator::Eq) => Ok(Box::new(
            function_hashes::function_hash.eq(value_owned.clone()),
        )),
        ("function_hash", FilterOperator::Like) => {
            let pattern = format!("%{}%", value_owned);
            Ok(Box::new(function_hashes::function_hash.like(pattern)))
        }

        // Nullable description column
        ("description", FilterOperator::Eq) => Ok(Box::new(
            function_hashes::description
                .eq(value_owned.clone())
                .assume_not_null(),
        )),
        ("description", FilterOperator::Like) => {
            let pattern = format!("%{}%", value_owned);
            Ok(Box::new(
                function_hashes::description.like(pattern).assume_not_null(),
            ))
        }

        // Source filters
        ("source", FilterOperator::Eq) => Ok(Box::new(
            function_hashes::source
                .eq(value_owned.clone())
                .assume_not_null(),
        )),
        ("source", FilterOperator::Like) => {
            let pattern = format!("%{}%", value_owned);
            Ok(Box::new(
                Box::new(function_hashes::source.like(pattern)).assume_not_null(),
            ))
        }

        ("created_at", FilterOperator::Eq) => Ok(Box::new(
            function_hashes::created_at.eq(value_owned.clone()),
        )),
        ("created_at", FilterOperator::Gt) => Ok(Box::new(
            function_hashes::created_at.gt(value_owned.clone()),
        )),
        ("created_at", FilterOperator::Lt) => Ok(Box::new(
            function_hashes::created_at.lt(value_owned.clone()),
        )),
        ("updated_at", FilterOperator::Eq) => Ok(Box::new(
            function_hashes::updated_at
                .eq(value_owned.clone())
                .assume_not_null(),
        )),
        ("updated_at", FilterOperator::Gt) => Ok(Box::new(
            function_hashes::updated_at
                .gt(value_owned.clone())
                .assume_not_null(),
        )),
        ("updated_at", FilterOperator::Lt) => Ok(Box::new(
            function_hashes::updated_at
                .lt(value_owned.clone())
                .assume_not_null(),
        )),

        _ => Err(DatabaseError::InvalidInput(format!(
            "Unsupported filter: {} with operator {:?}",
            field, operator
        ))),
    }
}

impl RepositoryDynamicQuery<FunctionHash> for FunctionHashRepository {
    /// Get Connection Stats records filtered by dynamic query parameters
    fn get_by_dynamic_query(
        &self,
        conn: &mut SqliteConnection,
        filters: &Vec<FilterCondition>,
        sort: Option<&Vec<SortCondition>>,
        page_size: i64,
        page: i64,
        registration_id: String,
    ) -> Result<(Vec<FunctionHash>, i64)> {
        debug!(
            filter_count = filters.len(),
            page, page_size, "Querying function hashes"
        );

        // Build the base query
        let mut sql_query = function_hashes::table
            .select(FunctionHash::as_select())
            .into_boxed();

        // Build the count query
        let mut count_query = function_hashes::table
            .select(FunctionHash::as_select())
            .into_boxed();

        // Apply filter conditions to both queries
        for filter in filters {
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
                    ("id", SortDirection::Asc) => sql_query.order_by(function_hashes::id.asc()),
                    ("id", SortDirection::Desc) => sql_query.order_by(function_hashes::id.desc()),
                    ("function_hash", SortDirection::Asc) => {
                        sql_query.order_by(function_hashes::function_hash.asc())
                    }
                    ("function_hash", SortDirection::Desc) => {
                        sql_query.order_by(function_hashes::function_hash.desc())
                    }
                    ("description", SortDirection::Asc) => {
                        sql_query.order_by(function_hashes::description.asc())
                    }
                    ("description", SortDirection::Desc) => {
                        sql_query.order_by(function_hashes::description.desc())
                    }
                    ("source", SortDirection::Asc) => {
                        sql_query.order_by(function_hashes::source.asc())
                    }
                    ("source", SortDirection::Desc) => {
                        sql_query.order_by(function_hashes::source.desc())
                    }
                    ("created_at", SortDirection::Asc) => {
                        sql_query.order_by(function_hashes::created_at.asc())
                    }
                    ("created_at", SortDirection::Desc) => {
                        sql_query.order_by(function_hashes::created_at.desc())
                    }
                    ("updated_at", SortDirection::Asc) => {
                        sql_query.order_by(function_hashes::updated_at.asc())
                    }
                    ("updated_at", SortDirection::Desc) => {
                        sql_query.order_by(function_hashes::updated_at.desc())
                    }
                    _ => sql_query,
                };
            }
        }

        #[cfg(debug_assertions)]
        trace!(query=%debug_query(&count_query),"Retrieving count of function hash records from database");

        // count the filtered results
        let total_count: i64 = count_query.count().get_result(conn)?;

        // Apply pagination
        if page > 0 && page_size > 0 {
            sql_query = sql_query.limit(page_size).offset((page - 1) * page_size);
        }

        #[cfg(debug_assertions)]
        trace!(query=%debug_query(&sql_query),"Retrieving function hash records from database");

        // Execute
        let function_hash_records = sql_query.load::<FunctionHash>(conn)?;
        info!(
            total = total_count,
            returned = function_hash_records.len(),
            "Function hashes query completed"
        );

        Ok((function_hash_records, total_count))
    }

    fn delete_by_dynamic_query(
        &self,
        conn: &mut SqliteConnection,
        query: &DeleteQuery,
        registration_id: String,
    ) -> Result<usize> {
        debug!(
            filter_count = query.filters.len(),
            "Deleting function hashes by query"
        );
        match self.get_by_dynamic_query(conn, &query.filters, None, 0, 0, registration_id) {
            Ok((connection_stat_records, num_records)) => {
                let record_count = num_records as usize;
                let result = conn
                    .transaction(|conn| {
                        for item in connection_stat_records {
                            diesel::delete(
                                function_hashes::table.filter(function_hashes::id.eq(item.id)),
                            )
                            .execute(conn)?;
                        }

                        Ok(record_count)
                    })
                    .map_err(|error: diesel::result::Error| DatabaseError::from(error));

                if result.is_ok() {
                    info!(deleted = record_count, "Function hashes deleted");
                } else {
                    warn!(count = record_count, "Failed to delete function hashes");
                }
                result
            }
            Err(error) => {
                warn!("Failed to query function hashes for deletion");
                Err(error)
            }
        }
    }
}

impl RepositoryGenericInsert<FunctionHash, NewFunctionHash> for FunctionHashRepository {
    fn create(&self, conn: &mut SqliteConnection, item: NewFunctionHash) -> Result<FunctionHash> {
        debug!(hash = %item.function_hash, "Creating function hash");
        let new_function_hash = diesel::insert_into(function_hashes::table)
            .values(&item)
            .returning(FunctionHash::as_returning())
            .get_result(conn)?;
        info!(id = new_function_hash.id, hash = %new_function_hash.function_hash, "Function hash created");
        Ok(new_function_hash)
    }

    fn create_many(
        &self,
        conn: &mut SqliteConnection,
        items: Vec<NewFunctionHash>,
    ) -> Result<Vec<FunctionHash>> {
        use diesel::Connection;

        debug!(count = items.len(), "Creating multiple function hashes");
        let result = conn.transaction(|conn| {
            let mut function_hashes: Vec<FunctionHash> = Vec::new();

            for item in items {
                let function_hash_record = diesel::insert_into(function_hashes::table)
                    .values(&item)
                    .returning(FunctionHash::as_returning())
                    .get_result(conn)?;

                function_hashes.push(function_hash_record)
            }

            Ok(function_hashes)
        });

        match &result {
            Ok(hashes) => info!(count = hashes.len(), "Function hashes created"),
            Err(_) => warn!("Failed to create function hashes"),
        }

        result.map_err(|error: diesel::result::Error| DatabaseError::from(error))
    }
}

impl RepositoryGenericUpdate<FunctionHash, UpdateFunctionHash> for FunctionHashRepository {
    fn update(
        &self,
        conn: &mut SqliteConnection,
        item: UpdateFunctionHash,
    ) -> Result<FunctionHash> {
        debug!(id = item.id, "Updating function hash");
        let updated = diesel::update(function_hashes::table)
            .filter(function_hashes::id.eq(item.id))
            .set(&item)
            .returning(FunctionHash::as_returning())
            .get_result(conn)?;
        info!(id = updated.id, "Function hash updated");
        Ok(updated)
    }
}
