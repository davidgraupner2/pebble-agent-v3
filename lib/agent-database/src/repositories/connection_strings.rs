use std::str::FromStr;

use crate::errors::Result;
use crate::models::{ConnectionString, ConnectionStringStatus, NewConnectionString};
use crate::query::{DeleteQuery, FilterCondition, FilterOperator, SortCondition, SortDirection};
use crate::schema::connection_strings;
use crate::traits::RepositoryGenericInsert;
use crate::{
    DatabaseError, RepositoryDynamicQuery, RepositoryGenericUpdate, UpdateConnectionString,
};
use diesel::sql_types::Bool;
use diesel::sqlite::Sqlite;
use diesel::{debug_query, prelude::*};
use tracing::{debug, info, warn};

#[cfg(debug_assertions)]
use tracing::trace;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ConnectionStringRepository {}

impl ConnectionStringRepository {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

// Need this type for common condition expressions
type BoxedCondition = Box<dyn BoxableExpression<connection_strings::table, Sqlite, SqlType = Bool>>;

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
            connection_strings::id.eq(value_owned.parse::<i32>()?),
        )),
        ("id", FilterOperator::Gt) => Ok(Box::new(
            connection_strings::id.gt(value_owned.parse::<i32>()?),
        )),
        ("id", FilterOperator::Gte) => Ok(Box::new(
            connection_strings::id.ge(value_owned.parse::<i32>()?),
        )),
        ("id", FilterOperator::Lt) => Ok(Box::new(
            connection_strings::id.lt(value_owned.parse::<i32>()?),
        )),
        ("id", FilterOperator::Lte) => Ok(Box::new(
            connection_strings::id.le(value_owned.parse::<i32>()?),
        )),

        // Value filters
        ("value", FilterOperator::Eq) => {
            Ok(Box::new(connection_strings::value.eq(value_owned.clone())))
        }
        ("value", FilterOperator::Like) => {
            let pattern = format!("%{}%", value_owned);
            Ok(Box::new(connection_strings::value.like(pattern)))
        }

        // Description Filters
        ("description", FilterOperator::Eq) => Ok(Box::new(
            connection_strings::description
                .eq(value_owned.clone())
                .assume_not_null(),
        )),
        ("description", FilterOperator::Like) => {
            let pattern = format!("%{}%", value_owned);
            Ok(Box::new(
                connection_strings::description
                    .like(pattern)
                    .assume_not_null(),
            ))
        }

        // Source filters
        ("source", FilterOperator::Eq) => {
            Ok(Box::new(connection_strings::source.eq(value_owned.clone())))
        }
        ("source", FilterOperator::Like) => {
            let pattern = format!("%{}%", value_owned);
            Ok(Box::new(connection_strings::source.like(pattern)))
        }

        ("status", FilterOperator::Eq) => {
            let status = ConnectionStringStatus::from_str(&value_owned)
                .map_err(|e| DatabaseError::InvalidInput(format!("Invalid status: {}", e)))?;
            Ok(Box::new(connection_strings::status.eq(status)))
        }
        ("status", FilterOperator::Like) => {
            // Since ConnectionStringStatus has a fixed set of values, filter by matching substring
            let pattern_lower = value_owned.to_lowercase();

            // Build OR condition for all matching statuses
            let mut condition_built = false;
            let mut condition: Option<BoxedCondition> = None;

            for status in &[
                ConnectionStringStatus::Pending,
                ConnectionStringStatus::InUse,
            ] {
                let status_str = format!("{:?}", status).to_lowercase();
                let status_str = if status_str == "inuse" {
                    "in_use".to_string()
                } else {
                    status_str
                };

                if status_str.contains(&pattern_lower) {
                    if !condition_built {
                        condition = Some(Box::new(connection_strings::status.eq(*status)));
                        condition_built = true;
                    } else {
                        condition = Some(Box::new(
                            condition
                                .unwrap()
                                .or(connection_strings::status.eq(*status)),
                        ));
                    }
                }
            }

            match condition {
                Some(cond) => Ok(cond),
                None => Ok(Box::new(connection_strings::id.eq(-1))), // No matches = impossible condition
            }
        }
        ("created_at", FilterOperator::Eq) => Ok(Box::new(
            connection_strings::created_at.eq(value_owned.clone()),
        )),
        ("created_at", FilterOperator::Gt) => Ok(Box::new(
            connection_strings::created_at.gt(value_owned.clone()),
        )),
        ("created_at", FilterOperator::Lt) => Ok(Box::new(
            connection_strings::created_at.lt(value_owned.clone()),
        )),
        ("updated_at", FilterOperator::Eq) => Ok(Box::new(
            connection_strings::updated_at
                .eq(value_owned.clone())
                .assume_not_null(),
        )),
        ("updated_at", FilterOperator::Gt) => Ok(Box::new(
            connection_strings::updated_at
                .gt(value_owned.clone())
                .assume_not_null(),
        )),
        ("updated_at", FilterOperator::Lt) => Ok(Box::new(
            connection_strings::updated_at
                .lt(value_owned.clone())
                .assume_not_null(),
        )),

        _ => Err(DatabaseError::InvalidInput(format!(
            "Unsupported filter: {} with operator {:?}",
            field, operator
        ))),
    }
}

impl RepositoryDynamicQuery<ConnectionString> for ConnectionStringRepository {
    /// Get connection string records filtered by dynamic query parameters
    fn get_by_dynamic_query(
        &self,
        conn: &mut SqliteConnection,
        filters: &Vec<FilterCondition>,
        sort: Option<&Vec<SortCondition>>,
        page_size: i64,
        page: i64,
        registration_id: String,
    ) -> Result<(Vec<ConnectionString>, i64)> {
        debug!(
            filter_count = filters.len(),
            page, page_size, "Querying connection strings"
        );

        // Build the base query
        let mut sql_query = connection_strings::table
            .select(ConnectionString::as_select())
            .into_boxed();

        // Build the count query
        let mut count_query = connection_strings::table
            .select(ConnectionString::as_select())
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
                    ("id", SortDirection::Asc) => sql_query.order_by(connection_strings::id.asc()),
                    ("id", SortDirection::Desc) => {
                        sql_query.order_by(connection_strings::id.desc())
                    }
                    ("value", SortDirection::Asc) => {
                        sql_query.order_by(connection_strings::value.asc())
                    }
                    ("value", SortDirection::Desc) => {
                        sql_query.order_by(connection_strings::value.desc())
                    }
                    ("description", SortDirection::Asc) => {
                        sql_query.order_by(connection_strings::description.asc())
                    }
                    ("description", SortDirection::Desc) => {
                        sql_query.order_by(connection_strings::description.desc())
                    }
                    ("source", SortDirection::Asc) => {
                        sql_query.order_by(connection_strings::source.asc())
                    }
                    ("source", SortDirection::Desc) => {
                        sql_query.order_by(connection_strings::source.desc())
                    }
                    ("status", SortDirection::Asc) => {
                        sql_query.order_by(connection_strings::status.asc())
                    }
                    ("status", SortDirection::Desc) => {
                        sql_query.order_by(connection_strings::status.desc())
                    }
                    ("created_at", SortDirection::Asc) => {
                        sql_query.order_by(connection_strings::created_at.asc())
                    }
                    ("created_at", SortDirection::Desc) => {
                        sql_query.order_by(connection_strings::created_at.desc())
                    }
                    ("updated_at", SortDirection::Asc) => {
                        sql_query.order_by(connection_strings::updated_at.asc())
                    }
                    ("updated_at", SortDirection::Desc) => {
                        sql_query.order_by(connection_strings::updated_at.desc())
                    }
                    _ => sql_query,
                };
            }
        }

        #[cfg(debug_assertions)]
        trace!(query=%debug_query(&count_query),"Retrieving count of connection_string records from database");

        // count the filtered results
        let total_count: i64 = count_query.count().get_result(conn)?;

        // Apply pagination
        if page > 0 && page_size > 0 {
            sql_query = sql_query.limit(page_size).offset((page - 1) * page_size);
        }

        #[cfg(debug_assertions)]
        trace!(query=%debug_query(&sql_query),"Retrieving connection strings from database");

        // Execute
        let connection_string_records = sql_query.load::<ConnectionString>(conn)?;

        info!(
            total = total_count,
            returned = connection_string_records.len(),
            "Connection strings query completed"
        );

        Ok((connection_string_records, total_count))
    }

    fn delete_by_dynamic_query(
        &self,
        conn: &mut SqliteConnection,
        query: &DeleteQuery,
        registration_id: String,
    ) -> Result<usize> {
        debug!(
            filter_count = query.filters.len(),
            "Deleting connection strings by query"
        );

        match self.get_by_dynamic_query(conn, &query.filters, None, 0, 0, registration_id) {
            Ok((connection_string_records, num_records)) => {
                let record_count = num_records as usize;
                let result = conn
                    .transaction(|conn| {
                        for item in connection_string_records {
                            diesel::delete(
                                connection_strings::table
                                    .filter(connection_strings::id.eq(item.id)),
                            )
                            .execute(conn)?;
                        }

                        Ok(record_count)
                    })
                    .map_err(|error: diesel::result::Error| DatabaseError::from(error));

                if result.is_ok() {
                    info!(deleted = record_count, "Connection strings deleted");
                } else {
                    warn!(count = record_count, "Failed to delete connection strings");
                }
                result
            }
            Err(error) => {
                warn!("Failed to query connection strings for deletion");
                Err(error)
            }
        }
    }
}

impl RepositoryGenericInsert<ConnectionString, NewConnectionString> for ConnectionStringRepository {
    fn create(
        &self,
        conn: &mut SqliteConnection,
        item: NewConnectionString,
    ) -> Result<ConnectionString> {
        debug!("Creating connection string");
        #[cfg(debug_assertions)]
        trace!(value = %item.value, "Connection string details");

        let inserted = diesel::insert_into(connection_strings::table)
            .values(&item)
            .returning(ConnectionString::as_returning())
            .get_result(conn)?;
        info!(id = inserted.id, "Connection string created");
        Ok(inserted)
    }

    fn create_many(
        &self,
        conn: &mut SqliteConnection,
        items: Vec<NewConnectionString>,
    ) -> Result<Vec<ConnectionString>> {
        use diesel::Connection;

        debug!(count = items.len(), "Creating multiple connection strings");
        #[cfg(debug_assertions)]
        {
            for item in &items {
                trace!(value = %item.value, "Creating connection string with details");
            }
        }

        let result = conn.transaction(|conn| {
            let mut connection_strings: Vec<ConnectionString> = Vec::new();

            for item in items {
                let connection_string = diesel::insert_into(connection_strings::table)
                    .values(&item)
                    .returning(ConnectionString::as_returning())
                    .get_result(conn)?;

                connection_strings.push(connection_string)
            }

            Ok(connection_strings)
        });

        match &result {
            Ok(strings) => info!(count = strings.len(), "Connection strings created"),
            Err(_) => warn!("Failed to create connection strings"),
        }

        result.map_err(|error: diesel::result::Error| DatabaseError::from(error))
    }
}

impl RepositoryGenericUpdate<ConnectionString, UpdateConnectionString>
    for ConnectionStringRepository
{
    fn update(
        &self,
        conn: &mut SqliteConnection,
        update: UpdateConnectionString,
    ) -> Result<ConnectionString> {
        debug!(id = update.id, "Updating connection string record");
        let updated = diesel::update(connection_strings::table)
            .set(&update)
            .filter(connection_strings::id.eq(update.id))
            .returning(ConnectionString::as_returning())
            .get_result(conn)?;
        info!(id = updated.id, "Connection string record updated");
        Ok(updated)
    }
}
