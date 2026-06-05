use crate::RepositoryDynamicQuery;
use crate::errors::{DatabaseError, Result};
use crate::models::{Event, EventStatus, NewEvent, UpdateEvent};
use crate::query::{DeleteQuery, FilterCondition, FilterOperator, SortCondition, SortDirection};
use crate::schema::events::{self};
use crate::traits::{RepositoryGenericInsert, RepositoryGenericUpdate};
use diesel::sql_types::Bool;
use diesel::sqlite::Sqlite;
use diesel::{debug_query, prelude::*};
use std::str::FromStr;
use tracing::{debug, info, warn};

#[cfg(debug_assertions)]
use tracing::trace;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct EventRepository;

impl EventRepository {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

// Need this type for common condition expressions
type BoxedCondition = Box<dyn BoxableExpression<events::table, Sqlite, SqlType = Bool>>;

/// Build a single boxed condition from a filter
fn build_filter_condition(
    field: &str,
    operator: &FilterOperator,
    value: &str,
) -> Result<BoxedCondition> {
    let value_owned = value.to_string();
    match (field, operator) {
        // Non-nullable columns (these are fine as-is)
        ("id", FilterOperator::Eq) => Ok(Box::new(events::id.eq(value_owned.parse::<i32>()?))),
        ("id", FilterOperator::Gt) => Ok(Box::new(events::id.gt(value_owned.parse::<i32>()?))),
        ("id", FilterOperator::Gte) => Ok(Box::new(events::id.ge(value_owned.parse::<i32>()?))),
        ("id", FilterOperator::Lt) => Ok(Box::new(events::id.lt(value_owned.parse::<i32>()?))),
        ("id", FilterOperator::Lte) => Ok(Box::new(events::id.le(value_owned.parse::<i32>()?))),

        // Event Type filters
        ("event_type", FilterOperator::Eq) => {
            Ok(Box::new(events::event_type.eq(value_owned.clone())))
        }
        ("event_type", FilterOperator::Like) => {
            let pattern = format!("%{}%", value_owned);
            Ok(Box::new(events::event_type.like(pattern)))
        }

        // Aggregate Type filters
        ("aggregate_type", FilterOperator::Eq) => {
            Ok(Box::new(events::aggregate_type.eq(value_owned.clone())))
        }
        ("aggregate_type", FilterOperator::Like) => {
            let pattern = format!("%{}%", value_owned);
            Ok(Box::new(events::aggregate_type.like(pattern)))
        }

        // Aggregate ID filters
        ("aggregate_id", FilterOperator::Eq) => {
            Ok(Box::new(events::aggregate_id.eq(value_owned.clone())))
        }
        ("aggregate_id", FilterOperator::Like) => {
            let pattern = format!("%{}%", value_owned);
            Ok(Box::new(events::aggregate_id.like(pattern)))
        }

        // Payload filters
        ("payload", FilterOperator::Eq) => Ok(Box::new(events::payload.eq(value_owned.clone()))),
        ("payload", FilterOperator::Like) => {
            let pattern = format!("%{}%", value_owned);
            Ok(Box::new(events::payload.like(pattern)))
        }

        // Nullable columns - ALL need .assume_not_null()
        ("metadata", FilterOperator::Eq) => Ok(Box::new(
            events::metadata.eq(value_owned.clone()).assume_not_null(),
        )),
        ("metadata", FilterOperator::Like) => {
            let pattern = format!("%{}%", value_owned);
            Ok(Box::new(events::metadata.like(pattern).assume_not_null()))
        }

        // Event Status filters
        ("status", FilterOperator::Eq) => {
            let status = EventStatus::from_str(&value_owned)
                .map_err(|e| DatabaseError::InvalidInput(format!("Invalid status: {}", e)))?;
            Ok(Box::new(events::status.eq(status)))
        }
        ("status", FilterOperator::Like) => {
            // Since EventStatus has a fixed set of values, filter by matching substring
            let pattern_lower = value_owned.to_lowercase();

            // Build OR condition for all matching statuses
            let mut condition_built = false;
            let mut condition: Option<BoxedCondition> = None;

            for status in &[
                EventStatus::Pending,
                EventStatus::InProgress,
                EventStatus::Completed,
                EventStatus::Failed,
            ] {
                if format!("{:?}", status)
                    .to_lowercase()
                    .contains(&pattern_lower)
                {
                    if !condition_built {
                        condition = Some(Box::new(events::status.eq(*status)));
                        condition_built = true;
                    } else {
                        condition =
                            Some(Box::new(condition.unwrap().or(events::status.eq(*status))));
                    }
                }
            }

            match condition {
                Some(cond) => Ok(cond),
                None => Ok(Box::new(events::id.eq(-1))), // No matches = impossible condition
            }
        }

        // Created At filters
        ("created_at", FilterOperator::Eq) => {
            Ok(Box::new(events::created_at.eq(value_owned.clone())))
        }
        ("created_at", FilterOperator::Gt) => {
            Ok(Box::new(events::created_at.gt(value_owned.clone())))
        }
        ("created_at", FilterOperator::Lt) => {
            Ok(Box::new(events::created_at.lt(value_owned.clone())))
        }

        // Processed At filters
        ("processed_at", FilterOperator::Eq) => Ok(Box::new(
            events::processed_at
                .eq(value_owned.clone())
                .assume_not_null(),
        )),
        ("processed_at", FilterOperator::Gt) => Ok(Box::new(
            events::processed_at
                .gt(value_owned.clone())
                .assume_not_null(),
        )),
        ("processed_at", FilterOperator::Lt) => Ok(Box::new(
            events::processed_at
                .lt(value_owned.clone())
                .assume_not_null(),
        )),

        // Retry Count filters
        ("retry_count", FilterOperator::Eq) => Ok(Box::new(
            events::retry_count.eq(value_owned.parse::<i32>()?),
        )),
        ("retry_count", FilterOperator::Gt) => Ok(Box::new(
            events::retry_count.gt(value_owned.parse::<i32>()?),
        )),
        ("retry_count", FilterOperator::Lt) => Ok(Box::new(
            events::retry_count.lt(value_owned.parse::<i32>()?),
        )),

        _ => Err(DatabaseError::InvalidInput(format!(
            "Unsupported filter: {} with operator {:?}",
            field, operator
        ))),
    }
}

impl RepositoryDynamicQuery<Event> for EventRepository {
    /// Get cache records filtered by dynamic query parameters
    fn get_by_dynamic_query(
        &self,
        conn: &mut SqliteConnection,
        filters: &Vec<FilterCondition>,
        sort: Option<&Vec<SortCondition>>,
        page_size: i64,
        page: i64,
    ) -> Result<(Vec<Event>, i64)> {
        debug!(
            filter_count = filters.len(),
            page, page_size, "Querying events"
        );

        // Build the base query
        let mut sql_query = events::table.select(Event::as_select()).into_boxed();

        // Build the count query
        let mut count_query = events::table.select(Event::as_select()).into_boxed();

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
                    ("id", SortDirection::Asc) => sql_query.order_by(events::id.asc()),
                    ("id", SortDirection::Desc) => sql_query.order_by(events::id.desc()),
                    ("event_type", SortDirection::Asc) => {
                        sql_query.order_by(events::event_type.asc())
                    }
                    ("event_type", SortDirection::Desc) => {
                        sql_query.order_by(events::event_type.desc())
                    }
                    ("aggregate_type", SortDirection::Asc) => {
                        sql_query.order_by(events::aggregate_type.asc())
                    }
                    ("aggregate_type", SortDirection::Desc) => {
                        sql_query.order_by(events::aggregate_type.desc())
                    }
                    ("aggregate_id", SortDirection::Asc) => {
                        sql_query.order_by(events::aggregate_id.asc())
                    }
                    ("aggregate_id", SortDirection::Desc) => {
                        sql_query.order_by(events::aggregate_id.desc())
                    }
                    ("payload", SortDirection::Asc) => sql_query.order_by(events::payload.asc()),
                    ("payload", SortDirection::Desc) => sql_query.order_by(events::payload.desc()),
                    ("metadata", SortDirection::Asc) => sql_query.order_by(events::metadata.asc()),
                    ("metadata", SortDirection::Desc) => {
                        sql_query.order_by(events::metadata.desc())
                    }
                    ("status", SortDirection::Asc) => sql_query.order_by(events::status.asc()),
                    ("status", SortDirection::Desc) => sql_query.order_by(events::status.desc()),
                    ("retry_count", SortDirection::Asc) => {
                        sql_query.order_by(events::retry_count.asc())
                    }
                    ("retry_count", SortDirection::Desc) => {
                        sql_query.order_by(events::retry_count.desc())
                    }
                    ("processed_at", SortDirection::Asc) => {
                        sql_query.order_by(events::processed_at.asc())
                    }
                    ("processed_at", SortDirection::Desc) => {
                        sql_query.order_by(events::processed_at.desc())
                    }
                    ("created_at", SortDirection::Asc) => {
                        sql_query.order_by(events::created_at.asc())
                    }
                    ("created_at", SortDirection::Desc) => {
                        sql_query.order_by(events::created_at.desc())
                    }
                    _ => sql_query,
                };
            }
        }

        #[cfg(debug_assertions)]
        trace!(query=%debug_query(&count_query),"Retrieving count of event records from database");

        // count the filtered results
        let total_count: i64 = count_query.count().get_result(conn)?;

        // Apply pagination
        if page > 0 && page_size > 0 {
            sql_query = sql_query.limit(page_size).offset((page - 1) * page_size);
        }

        #[cfg(debug_assertions)]
        trace!(query=%debug_query(&sql_query),"Retrieving event records from database");

        // Execute
        let event_records = sql_query.load::<Event>(conn)?;
        info!(
            total = total_count,
            returned = event_records.len(),
            "Events query completed"
        );

        Ok((event_records, total_count))
    }

    fn delete_by_dynamic_query(
        &self,
        conn: &mut SqliteConnection,
        query: &DeleteQuery,
    ) -> Result<usize> {
        debug!(
            filter_count = query.filters.len(),
            "Deleting events by query"
        );
        match self.get_by_dynamic_query(conn, &query.filters, None, 0, 0) {
            Ok((event_records, num_records)) => {
                let record_count = num_records as usize;
                let result = conn
                    .transaction(|conn| {
                        for item in event_records {
                            diesel::delete(events::table.filter(events::id.eq(item.id)))
                                .execute(conn)?;
                        }

                        Ok(record_count)
                    })
                    .map_err(|error: diesel::result::Error| DatabaseError::from(error));

                if result.is_ok() {
                    info!(deleted = record_count, "Events deleted");
                } else {
                    warn!(count = record_count, "Failed to delete events");
                }
                result
            }
            Err(error) => {
                warn!("Failed to query events for deletion");
                Err(error)
            }
        }
    }
}

impl RepositoryGenericInsert<Event, NewEvent> for EventRepository {
    fn create(&self, conn: &mut SqliteConnection, item: NewEvent) -> Result<Event> {
        debug!(aggregate_type = %item.aggregate_type, event_type = %item.event_type, "Creating event");
        let inserted_event = diesel::insert_into(events::table)
            .values(&item)
            .returning(Event::as_returning())
            .get_result(conn)?;
        info!(id = inserted_event.id, aggregate_type = %inserted_event.aggregate_type, "Event created");
        Ok(inserted_event)
    }

    fn create_many(&self, conn: &mut SqliteConnection, items: Vec<NewEvent>) -> Result<Vec<Event>> {
        use diesel::Connection;

        debug!(count = items.len(), "Creating multiple events");
        let result = conn.transaction(|conn| {
            let mut events: Vec<Event> = Vec::new();

            for item in items {
                let event = diesel::insert_into(events::table)
                    .values(&item)
                    .returning(Event::as_returning())
                    .get_result(conn)?;

                events.push(event)
            }

            Ok(events)
        });

        match &result {
            Ok(events) => info!(count = events.len(), "Events created"),
            Err(_) => warn!("Failed to create events"),
        }

        result.map_err(|error: diesel::result::Error| DatabaseError::from(error))
    }
}

impl RepositoryGenericUpdate<Event, UpdateEvent> for EventRepository {
    fn update(&self, conn: &mut SqliteConnection, update: UpdateEvent) -> Result<Event> {
        debug!(id = update.id, "Updating event");
        let updated_event = diesel::update(events::table)
            .filter(events::id.eq(update.id))
            .set(&update)
            .returning(Event::as_returning())
            .get_result(conn)?;
        info!(id = updated_event.id, status = ?updated_event.status, "Event updated");
        Ok(updated_event)
    }
}
