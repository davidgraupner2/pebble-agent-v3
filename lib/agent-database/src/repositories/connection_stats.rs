// use crate::schema::sql_types::ConnectionStatsStatus; // No longer needed
use crate::errors::{DatabaseError, Result};
use crate::models::{
    ConnectionStats, ConnectionStatsStatus, NewConnectionStats, UpdateConnectionStats,
};
use crate::query::{DeleteQuery, FilterCondition, FilterOperator, SortCondition, SortDirection};
use crate::schema::connection_stats::{self};
use crate::traits::RepositoryGenericInsert;
use crate::{RepositoryDynamicQuery, RepositoryGenericUpdate};
use diesel::sql_types::Bool;
use diesel::sqlite::Sqlite;
use diesel::{debug_query, prelude::*};
use std::str::FromStr;
use tracing::{debug, info, warn};

#[cfg(debug_assertions)]
use tracing::trace;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ConnectionStatsRepository {}

impl ConnectionStatsRepository {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

// Need this type for common condition expressions
type BoxedCondition = Box<dyn BoxableExpression<connection_stats::table, Sqlite, SqlType = Bool>>;

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
            connection_stats::id.eq(value_owned.parse::<i32>()?),
        )),
        ("id", FilterOperator::Gt) => Ok(Box::new(
            connection_stats::id.gt(value_owned.parse::<i32>()?),
        )),
        ("id", FilterOperator::Gte) => Ok(Box::new(
            connection_stats::id.ge(value_owned.parse::<i32>()?),
        )),
        ("id", FilterOperator::Lt) => Ok(Box::new(
            connection_stats::id.lt(value_owned.parse::<i32>()?),
        )),
        ("id", FilterOperator::Lte) => Ok(Box::new(
            connection_stats::id.le(value_owned.parse::<i32>()?),
        )),

        //Endpoint filters
        ("endpoint", FilterOperator::Eq) => {
            Ok(Box::new(connection_stats::endpoint.eq(value_owned.clone())))
        }
        ("endpoint", FilterOperator::Like) => {
            let pattern = format!("%{}%", value_owned);
            Ok(Box::new(connection_stats::endpoint.like(pattern)))
        }

        ("status", FilterOperator::Eq) => {
            let status = ConnectionStatsStatus::from_str(&value_owned)
                .map_err(|e| DatabaseError::InvalidInput(format!("Invalid status: {}", e)))?;
            Ok(Box::new(connection_stats::status.eq(status)))
        }
        ("status", FilterOperator::Like) => {
            // Since ConnectionStringStatus has a fixed set of values, filter by matching substring
            let pattern_lower = value_owned.to_lowercase();

            // Build OR condition for all matching statuses
            let mut condition_built = false;
            let mut condition: Option<BoxedCondition> = None;

            for status in &[
                ConnectionStatsStatus::Connected,
                ConnectionStatsStatus::Disconnected,
            ] {
                let status_str = format!("{:?}", status).to_lowercase();

                if status_str.contains(&pattern_lower) {
                    if !condition_built {
                        condition = Some(Box::new(connection_stats::status.eq(*status)));
                        condition_built = true;
                    } else {
                        condition = Some(Box::new(
                            condition.unwrap().or(connection_stats::status.eq(*status)),
                        ));
                    }
                }
            }

            match condition {
                Some(cond) => Ok(cond),
                None => Ok(Box::new(connection_stats::id.eq(-1))), // No matches = impossible condition
            }
        }

        // Connected At filters
        ("connected_at", FilterOperator::Eq) => Ok(Box::new(
            connection_stats::connected_at.eq(value_owned.clone()),
        )),
        ("connected_at", FilterOperator::Gt) => Ok(Box::new(
            connection_stats::connected_at.gt(value_owned.clone()),
        )),
        ("connected_at", FilterOperator::Lt) => Ok(Box::new(
            connection_stats::connected_at.lt(value_owned.clone()),
        )),

        // Disconnected_at At filters
        ("disconnected_at", FilterOperator::Eq) => Ok(Box::new(
            connection_stats::disconnected_at
                .eq(value_owned.clone())
                .assume_not_null(),
        )),
        ("disconnected_at", FilterOperator::Gt) => Ok(Box::new(
            connection_stats::disconnected_at
                .gt(value_owned.clone())
                .assume_not_null(),
        )),
        ("disconnected_at", FilterOperator::Lt) => Ok(Box::new(
            connection_stats::disconnected_at
                .lt(value_owned.clone())
                .assume_not_null(),
        )),

        _ => Err(DatabaseError::InvalidInput(format!(
            "Unsupported filter: {} with operator {:?}",
            field, operator
        ))),
    }
}

impl RepositoryDynamicQuery<ConnectionStats> for ConnectionStatsRepository {
    /// Get Connection Stats records filtered by dynamic query parameters
    fn get_by_dynamic_query(
        &self,
        conn: &mut SqliteConnection,
        filters: &Vec<FilterCondition>,
        sort: Option<&Vec<SortCondition>>,
        page_size: i64,
        page: i64,
        registration_id: String,
    ) -> Result<(Vec<ConnectionStats>, i64)> {
        debug!(
            filter_count = filters.len(),
            page, page_size, "Querying connection stats"
        );

        // Build the base query
        let mut sql_query = connection_stats::table
            .select(ConnectionStats::as_select())
            .into_boxed();

        // Build the count query
        let mut count_query = connection_stats::table
            .select(ConnectionStats::as_select())
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
                    ("id", SortDirection::Asc) => sql_query.order_by(connection_stats::id.asc()),
                    ("id", SortDirection::Desc) => sql_query.order_by(connection_stats::id.desc()),
                    ("endpoint", SortDirection::Asc) => {
                        sql_query.order_by(connection_stats::endpoint.asc())
                    }
                    ("endpoint", SortDirection::Desc) => {
                        sql_query.order_by(connection_stats::endpoint.desc())
                    }
                    ("status", SortDirection::Asc) => {
                        sql_query.order_by(connection_stats::status.asc())
                    }
                    ("status", SortDirection::Desc) => {
                        sql_query.order_by(connection_stats::status.desc())
                    }
                    ("connected_at", SortDirection::Asc) => {
                        sql_query.order_by(connection_stats::connected_at.asc())
                    }
                    ("connected_at", SortDirection::Desc) => {
                        sql_query.order_by(connection_stats::connected_at.desc())
                    }
                    ("disconnected_at", SortDirection::Asc) => {
                        sql_query.order_by(connection_stats::disconnected_at.asc())
                    }
                    ("disconnected_at", SortDirection::Desc) => {
                        sql_query.order_by(connection_stats::disconnected_at.desc())
                    }
                    _ => sql_query,
                };
            }
        }

        #[cfg(debug_assertions)]
        trace!(query=%debug_query(&count_query),"Retrieving count of connection_stat records from database");

        // count the filtered results
        let total_count: i64 = count_query.count().get_result(conn)?;

        // Apply pagination
        if page > 0 && page_size > 0 {
            sql_query = sql_query.limit(page_size).offset((page - 1) * page_size);
        }

        #[cfg(debug_assertions)]
        trace!(query=%debug_query(&sql_query),"Retrieving connection_stat records from database");

        // Execute
        let connection_stat_records = sql_query.load::<ConnectionStats>(conn)?;
        info!(
            total = total_count,
            returned = connection_stat_records.len(),
            "Connection stats query completed"
        );

        Ok((connection_stat_records, total_count))
    }

    fn delete_by_dynamic_query(
        &self,
        conn: &mut SqliteConnection,
        query: &DeleteQuery,
        registration_id: String,
    ) -> Result<usize> {
        debug!(
            filter_count = query.filters.len(),
            "Deleting connection stats by query"
        );
        match self.get_by_dynamic_query(conn, &query.filters, None, 0, 0, registration_id) {
            Ok((connection_stat_records, num_records)) => {
                let record_count = num_records as usize;
                let result = conn
                    .transaction(|conn| {
                        for item in connection_stat_records {
                            diesel::delete(
                                connection_stats::table.filter(connection_stats::id.eq(item.id)),
                            )
                            .execute(conn)?;
                        }

                        Ok(record_count)
                    })
                    .map_err(|error: diesel::result::Error| DatabaseError::from(error));

                if result.is_ok() {
                    info!(deleted = record_count, "Connection stats deleted");
                } else {
                    warn!(count = record_count, "Failed to delete connection stats");
                }
                result
            }
            Err(error) => {
                warn!("Failed to query connection stats for deletion");
                Err(error)
            }
        }
    }
}

impl RepositoryGenericInsert<ConnectionStats, NewConnectionStats> for ConnectionStatsRepository {
    fn create(
        &self,
        conn: &mut SqliteConnection,
        item: NewConnectionStats,
    ) -> Result<ConnectionStats> {
        debug!("Creating connection stat record");
        let inserted = diesel::insert_into(connection_stats::table)
            .values(&item)
            .returning(ConnectionStats::as_returning())
            .get_result(conn)?;
        info!(id = inserted.id, "Connection stat record created");
        Ok(inserted)
    }

    fn create_many(
        &self,
        conn: &mut SqliteConnection,
        items: Vec<NewConnectionStats>,
    ) -> Result<Vec<ConnectionStats>> {
        use diesel::Connection;

        debug!(
            count = items.len(),
            "Creating multiple connection stat records"
        );
        let result = conn.transaction(|conn| {
            let mut connection_stats: Vec<ConnectionStats> = Vec::new();

            for item in items.into_iter().enumerate() {
                let connection_stat = diesel::insert_into(connection_stats::table)
                    .values(&item.1)
                    .returning(ConnectionStats::as_returning())
                    .get_result(conn)?;

                connection_stats.push(connection_stat)
            }

            Ok(connection_stats)
        });

        match &result {
            Ok(stats) => info!(count = stats.len(), "Connection stat records created"),
            Err(_) => warn!("Failed to create connection stat records"),
        }

        result.map_err(|error: diesel::result::Error| DatabaseError::from(error))
    }
}

impl RepositoryGenericUpdate<ConnectionStats, UpdateConnectionStats> for ConnectionStatsRepository {
    fn update(
        &self,
        conn: &mut SqliteConnection,
        update: UpdateConnectionStats,
    ) -> Result<ConnectionStats> {
        debug!(id = update.id, "Updating connection stat record");
        let updated = diesel::update(connection_stats::table)
            .set(&update)
            .filter(connection_stats::id.eq(update.id))
            .returning(ConnectionStats::as_returning())
            .get_result(conn)?;
        info!(id = updated.id, "Connection stat record updated");
        Ok(updated)
    }
}
