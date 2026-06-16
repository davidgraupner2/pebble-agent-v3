use crate::errors::Result;
use crate::models::{NewRegistration, Registration};
use crate::query::{DeleteQuery, FilterCondition, FilterOperator, SortCondition, SortDirection};
use crate::schema::registration::{self};
use crate::traits::RepositoryGenericInsert;
use crate::{DatabaseError, RepositoryDynamicQuery};
use diesel::associations::HasTable;
use diesel::sql_types::Bool;
use diesel::sqlite::{Sqlite, SqliteConnection};
use diesel::{debug_query, prelude::*};
use tracing::{debug, info, warn};

#[cfg(debug_assertions)]
use tracing::trace;

type BoxedRegistrationCondition =
    Box<dyn BoxableExpression<registration::table, Sqlite, SqlType = Bool>>;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct RegistrationRepository;

impl RegistrationRepository {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

/// Build a single boxed condition from a filter
fn build_filter_condition(
    field: &str,
    operator: &FilterOperator,
    value: &str,
) -> Result<BoxedRegistrationCondition> {
    let value_owned = value.to_string();
    match (field, operator) {
        // ID Filters
        ("id", FilterOperator::Eq) => {
            Ok(Box::new(registration::id.eq(value_owned.parse::<i32>()?)))
        }
        ("id", FilterOperator::Gt) => {
            Ok(Box::new(registration::id.gt(value_owned.parse::<i32>()?)))
        }
        ("id", FilterOperator::Gte) => {
            Ok(Box::new(registration::id.ge(value_owned.parse::<i32>()?)))
        }
        ("id", FilterOperator::Lt) => {
            Ok(Box::new(registration::id.lt(value_owned.parse::<i32>()?)))
        }
        ("id", FilterOperator::Lte) => {
            Ok(Box::new(registration::id.le(value_owned.parse::<i32>()?)))
        }

        //Agent Id filters
        ("agent_id", FilterOperator::Eq) => {
            Ok(Box::new(registration::agent_id.eq(value_owned.clone())))
        }
        ("agent_id", FilterOperator::Like) => {
            let pattern = format!("%{}%", value_owned);
            Ok(Box::new(registration::agent_id.like(pattern)))
        }

        // jti description column
        ("jti", FilterOperator::Eq) => Ok(Box::new(
            registration::jti.eq(value_owned.clone()).assume_not_null(),
        )),
        ("jti", FilterOperator::Like) => {
            let pattern = format!("%{}%", value_owned);
            Ok(Box::new(registration::jti.like(pattern).assume_not_null()))
        }

        // Source filters
        ("source", FilterOperator::Eq) => Ok(Box::new(
            registration::source
                .eq(value_owned.clone())
                .assume_not_null(),
        )),
        ("source", FilterOperator::Like) => {
            let pattern = format!("%{}%", value_owned);
            Ok(Box::new(
                Box::new(registration::source.like(pattern)).assume_not_null(),
            ))
        }

        ("expires_at", FilterOperator::Eq) => Ok(Box::new(
            registration::expires_at
                .eq(value_owned.clone())
                .assume_not_null(),
        )),
        ("expires_at", FilterOperator::Gt) => Ok(Box::new(
            registration::expires_at
                .gt(value_owned.clone())
                .assume_not_null(),
        )),
        ("created_at", FilterOperator::Eq) => {
            Ok(Box::new(registration::created_at.eq(value_owned.clone())))
        }
        ("created_at", FilterOperator::Gt) => {
            Ok(Box::new(registration::created_at.gt(value_owned.clone())))
        }
        ("created_at", FilterOperator::Lt) => {
            Ok(Box::new(registration::created_at.lt(value_owned.clone())))
        }
        ("updated_at", FilterOperator::Eq) => Ok(Box::new(
            registration::updated_at
                .eq(value_owned.clone())
                .assume_not_null(),
        )),
        ("updated_at", FilterOperator::Gt) => Ok(Box::new(
            registration::updated_at
                .gt(value_owned.clone())
                .assume_not_null(),
        )),
        ("updated_at", FilterOperator::Lt) => Ok(Box::new(
            registration::updated_at
                .lt(value_owned.clone())
                .assume_not_null(),
        )),

        _ => Err(DatabaseError::InvalidInput(format!(
            "Unsupported filter: {} with operator {:?}",
            field, operator
        ))),
    }
}

impl RepositoryDynamicQuery<Registration> for RegistrationRepository {
    /// Get Connection Stats records filtered by dynamic query parameters
    fn get_by_dynamic_query(
        &self,
        conn: &mut SqliteConnection,
        filters: &Vec<FilterCondition>,
        sort: Option<&Vec<SortCondition>>,
        page_size: i64,
        page: i64,
        registration_id: String,
    ) -> Result<(Vec<Registration>, i64)> {
        debug!(
            filter_count = filters.len(),
            page, page_size, "Querying registrations"
        );

        // Build the base query
        let mut sql_query = Registration::table()
            .select(Registration::as_select())
            .into_boxed();

        // Build the count query
        let mut count_query = Registration::table()
            .select(Registration::as_select())
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
                    ("id", SortDirection::Asc) => sql_query.order_by(registration::id.asc()),
                    ("id", SortDirection::Desc) => sql_query.order_by(registration::id.desc()),
                    ("agent_id", SortDirection::Asc) => {
                        sql_query.order_by(registration::agent_id.asc())
                    }
                    ("agent_id", SortDirection::Desc) => {
                        sql_query.order_by(registration::agent_id.desc())
                    }
                    ("jti", SortDirection::Asc) => sql_query.order_by(registration::jti.asc()),
                    ("jti", SortDirection::Desc) => sql_query.order_by(registration::jti.desc()),
                    ("source", SortDirection::Asc) => {
                        sql_query.order_by(registration::source.asc())
                    }
                    ("source", SortDirection::Desc) => {
                        sql_query.order_by(registration::source.desc())
                    }
                    ("expires_at", SortDirection::Asc) => {
                        sql_query.order_by(registration::expires_at.asc())
                    }
                    ("expires_at", SortDirection::Desc) => {
                        sql_query.order_by(registration::expires_at.desc())
                    }
                    ("created_at", SortDirection::Asc) => {
                        sql_query.order_by(registration::created_at.asc())
                    }
                    ("created_at", SortDirection::Desc) => {
                        sql_query.order_by(registration::created_at.desc())
                    }
                    ("updated_at", SortDirection::Asc) => {
                        sql_query.order_by(registration::updated_at.asc())
                    }
                    ("updated_at", SortDirection::Desc) => {
                        sql_query.order_by(registration::updated_at.desc())
                    }
                    _ => sql_query,
                };
            }
        }

        #[cfg(debug_assertions)]
        trace!(query=%debug_query(&count_query),"Retrieving count of registration records from database");

        // count the filtered results
        let total_count: i64 = count_query.count().get_result(conn)?;

        // Apply pagination
        if page > 0 && page_size > 0 {
            sql_query = sql_query.limit(page_size).offset((page - 1) * page_size);
        }

        #[cfg(debug_assertions)]
        trace!(query=%debug_query(&sql_query),"Retrieving registration records from database");

        // Execute
        let registration_records = sql_query.load::<Registration>(conn)?;
        info!(
            total = total_count,
            returned = registration_records.len(),
            "Registrat query completed"
        );

        Ok((registration_records, total_count))
    }

    fn delete_by_dynamic_query(
        &self,
        conn: &mut SqliteConnection,
        query: &DeleteQuery,
        registration_id: String,
    ) -> Result<usize> {
        debug!(
            filter_count = query.filters.len(),
            "Deleting registrations by query"
        );
        match self.get_by_dynamic_query(conn, &query.filters, None, 0, 0, registration_id) {
            Ok((connection_stat_records, num_records)) => {
                let record_count = num_records as usize;
                let result = conn
                    .transaction(|conn| {
                        for item in connection_stat_records {
                            diesel::delete(
                                registration::table.filter(registration::id.eq(item.id)),
                            )
                            .execute(conn)?;
                        }

                        Ok(record_count)
                    })
                    .map_err(|error: diesel::result::Error| DatabaseError::from(error));

                if result.is_ok() {
                    info!(deleted = record_count, "Registrations deleted");
                } else {
                    warn!(count = record_count, "Failed to delete registrations");
                }
                result
            }
            Err(error) => {
                warn!("Failed to query registrations for deletion");
                Err(error)
            }
        }
    }
}

impl RepositoryGenericInsert<Registration, NewRegistration> for RegistrationRepository {
    fn create(&self, conn: &mut SqliteConnection, item: NewRegistration) -> Result<Registration> {
        debug!(registration = %item.agent_id, "Creating registration");
        let new_registration = diesel::insert_into(registration::table)
            .values(&item)
            .returning(Registration::as_returning())
            .get_result(conn)?;
        info!(id = new_registration.id, agent_id = %new_registration.agent_id, "Registration created");
        Ok(new_registration)
    }

    fn create_many(
        &self,
        conn: &mut SqliteConnection,
        items: Vec<NewRegistration>,
    ) -> Result<Vec<Registration>> {
        use diesel::Connection;

        debug!(count = items.len(), "Creating multiple registrations");
        let result = conn.transaction(|conn| {
            let mut registrations: Vec<Registration> = Vec::new();

            for item in items {
                let registration_record = diesel::insert_into(registration::table)
                    .values(&item)
                    .returning(Registration::as_returning())
                    .get_result(conn)?;

                registrations.push(registration_record)
            }

            Ok(registrations)
        });

        match &result {
            Ok(registrations) => info!(count = registrations.len(), "Registrations created"),
            Err(_) => warn!("Failed to create registrations"),
        }

        result.map_err(|error: diesel::result::Error| DatabaseError::from(error))
    }
}
