use crate::errors::Result;
use crate::models::{EncryptionKey, EncryptionKeyWithSecrets, NewEncryptionKey, Secret};
use crate::query::{DeleteQuery, FilterCondition, FilterOperator, SortCondition, SortDirection};
use crate::schema::{encryption_keys, secrets};
use crate::traits::{RepositoryByName, RepositoryDynamicQuery};
use crate::{
    DatabaseError, RepositoryGenericInsert, RepositoryGenericUpdate, UpdatedApiEncryptionKey,
};
use diesel::debug_query;
use diesel::sql_types::Bool;
use diesel::{prelude::*, sqlite::Sqlite};
use std::vec;
use tracing::{debug, info, warn};

#[cfg(debug_assertions)]
use tracing::trace;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct EncryptionKeyRepository;

impl EncryptionKeyRepository {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

// Need this type for common condition expressions
type BoxedCondition =
    Box<dyn BoxableExpression<encryption_keys::dsl::encryption_keys, Sqlite, SqlType = Bool>>;

/// Build a single boxed condition from a filter
fn build_filter_condition(
    field: &str,
    operator: &FilterOperator,
    value: &str,
) -> Result<BoxedCondition> {
    let value_owned = value.to_string();
    match (field, operator) {
        // ID filters
        ("id", FilterOperator::Eq) => {
            let id_val = value.parse::<i32>()?;
            Ok(Box::new(encryption_keys::id.eq(id_val)))
        }
        ("id", FilterOperator::Gt) => {
            let id_val = value.parse::<i32>()?;
            Ok(Box::new(encryption_keys::id.gt(id_val)))
        }
        ("id", FilterOperator::Lt) => {
            let id_val = value.parse::<i32>()?;
            Ok(Box::new(encryption_keys::id.lt(id_val)))
        }
        ("id", FilterOperator::Gte) => {
            let id_val = value.parse::<i32>()?;
            Ok(Box::new(encryption_keys::id.ge(id_val)))
        }
        ("id", FilterOperator::Lte) => {
            let id_val = value.parse::<i32>()?;
            Ok(Box::new(encryption_keys::id.le(id_val)))
        }

        // Name filters
        ("name", FilterOperator::Eq) => Ok(Box::new(encryption_keys::name.eq(value_owned.clone()))),
        ("name", FilterOperator::Like) => {
            let pattern = format!("%{}%", value_owned);
            Ok(Box::new(encryption_keys::name.like(pattern)))
        }

        // Public Key filters
        ("public_key", FilterOperator::Eq) => Ok(Box::new(
            encryption_keys::public_key.eq(value_owned.clone()),
        )),
        ("public_key", FilterOperator::Like) => {
            let pattern = format!("%{}%", value_owned);
            Ok(Box::new(encryption_keys::public_key.like(pattern)))
        }

        // Source filters
        ("source", FilterOperator::Eq) => {
            Ok(Box::new(encryption_keys::source.eq(value_owned.clone())))
        }
        ("source", FilterOperator::Like) => {
            let pattern = format!("%{}%", value_owned);
            Ok(Box::new(encryption_keys::source.like(pattern)))
        }

        // Enabled filters (boolean as integer: 0 or 1)
        ("enabled", FilterOperator::Eq) => {
            let enabled_val = match value.to_lowercase().as_str() {
                "true" | "1" | "yes" => 1,
                "false" | "0" | "no" => 0,
                _ => {
                    return Err(DatabaseError::InvalidInput(format!(
                        "Invalid boolean value for enabled: {}",
                        value
                    )));
                }
            };
            Ok(Box::new(encryption_keys::enabled.eq(enabled_val)))
        }

        // Created_at filters
        ("created_at", FilterOperator::Eq) => Ok(Box::new(
            encryption_keys::created_at.eq(value_owned.clone()),
        )),
        ("created_at", FilterOperator::Gt) => Ok(Box::new(
            encryption_keys::created_at.gt(value_owned.clone()),
        )),
        ("created_at", FilterOperator::Lt) => Ok(Box::new(
            encryption_keys::created_at.lt(value_owned.clone()),
        )),

        // Updated_at filters
        ("updated_at", FilterOperator::Eq) => Ok(Box::new(
            encryption_keys::updated_at.eq(value_owned.clone()),
        )),
        ("updated_at", FilterOperator::Gt) => Ok(Box::new(
            encryption_keys::updated_at.gt(value_owned.clone()),
        )),
        ("updated_at", FilterOperator::Lt) => Ok(Box::new(
            encryption_keys::updated_at.lt(value_owned.clone()),
        )),

        _ => Err(DatabaseError::InvalidInput(format!(
            "Unsupported filter: {} with operator {:?}",
            field, operator
        ))),
    }
}

impl RepositoryDynamicQuery<EncryptionKeyWithSecrets> for EncryptionKeyRepository {
    /// Get encryption keys filtered by dynamic query parameters
    fn get_by_dynamic_query(
        &self,
        conn: &mut SqliteConnection,
        // query: &DynamicQuery,
        filters: &Vec<FilterCondition>,
        sort: Option<&Vec<SortCondition>>,
        page_size: i64,
        page: i64,
    ) -> Result<(Vec<EncryptionKeyWithSecrets>, i64)> {
        debug!(
            filter_count = filters.len(),
            page, page_size, "Querying encryption keys"
        );

        // Build the base query
        let mut sql_query = encryption_keys::table
            .select(EncryptionKey::as_select())
            .into_boxed();

        // Build the count query
        let mut count_query = encryption_keys::table
            .select(EncryptionKey::as_select())
            .into_boxed();

        // Apply filter conditions to both queries
        for filter in filters {
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
                    ("id", SortDirection::Asc) => sql_query.order_by(encryption_keys::id.asc()),
                    ("id", SortDirection::Desc) => sql_query.order_by(encryption_keys::id.desc()),
                    ("name", SortDirection::Asc) => sql_query.order_by(encryption_keys::name.asc()),
                    ("name", SortDirection::Desc) => {
                        sql_query.order_by(encryption_keys::name.desc())
                    }
                    ("public_key", SortDirection::Asc) => {
                        sql_query.order_by(encryption_keys::public_key.asc())
                    }
                    ("public_key", SortDirection::Desc) => {
                        sql_query.order_by(encryption_keys::public_key.desc())
                    }
                    ("source", SortDirection::Asc) => {
                        sql_query.order_by(encryption_keys::source.asc())
                    }
                    ("source", SortDirection::Desc) => {
                        sql_query.order_by(encryption_keys::source.desc())
                    }
                    ("enabled", SortDirection::Asc) => {
                        sql_query.order_by(encryption_keys::enabled.asc())
                    }
                    ("enabled", SortDirection::Desc) => {
                        sql_query.order_by(encryption_keys::enabled.desc())
                    }
                    ("created_at", SortDirection::Asc) => {
                        sql_query.order_by(encryption_keys::created_at.asc())
                    }
                    ("created_at", SortDirection::Desc) => {
                        sql_query.order_by(encryption_keys::created_at.desc())
                    }
                    ("updated_at", SortDirection::Asc) => {
                        sql_query.order_by(encryption_keys::updated_at.asc())
                    }
                    ("updated_at", SortDirection::Desc) => {
                        sql_query.order_by(encryption_keys::updated_at.desc())
                    }
                    _ => sql_query,
                };
            }
        }

        #[cfg(debug_assertions)]
        trace!(query=%debug_query(&count_query),"Retrieving count of encryption key records from database");

        // count the filtered results
        let total_count: i64 = count_query.count().get_result(conn)?;

        // Apply pagination
        if page > 0 && page_size > 0 {
            sql_query = sql_query.limit(page_size).offset((page - 1) * page_size);
        }

        #[cfg(debug_assertions)]
        trace!(query=%debug_query(&sql_query),"Retrieving encryption key records from database");

        // Execute
        let encryption_keys = sql_query.load::<EncryptionKey>(conn)?;

        let secrets = fetch_secrets_for_encryption_keys(conn, &encryption_keys);

        let secrets_per_encryption_key =
            group_secrets_per_encryption_key(secrets, &encryption_keys);

        info!(
            total = total_count,
            returned = secrets_per_encryption_key.len(),
            "Encryption keys query completed"
        );
        Ok((secrets_per_encryption_key, total_count))
    }

    fn delete_by_dynamic_query(
        &self,
        conn: &mut SqliteConnection,
        query: &DeleteQuery,
    ) -> Result<usize> {
        debug!(
            filter_count = query.filters.len(),
            "Deleting encryption keys by query"
        );
        match self.get_by_dynamic_query(conn, &query.filters, None, 0, 0) {
            Ok((encrytion_keys, num_keys)) => {
                let key_count = num_keys as usize;
                let result = conn
                    .transaction(|conn| {
                        for item in encrytion_keys {
                            diesel::delete(
                                encryption_keys::table
                                    .filter(encryption_keys::id.eq(item.encryption_key.id)),
                            )
                            .execute(conn)?;
                        }

                        Ok(key_count)
                    })
                    .map_err(|error: diesel::result::Error| DatabaseError::from(error));

                if result.is_ok() {
                    info!(deleted = key_count, "Encryption keys deleted");
                } else {
                    warn!(count = key_count, "Failed to delete encryption keys");
                }
                result
            }
            Err(error) => {
                warn!("Failed to query encryption keys for deletion");
                Err(error)
            }
        }
    }
}

impl RepositoryGenericInsert<EncryptionKey, NewEncryptionKey> for EncryptionKeyRepository {
    fn create(&self, conn: &mut SqliteConnection, item: NewEncryptionKey) -> Result<EncryptionKey> {
        debug!(name = %item.name, "Creating encryption key");
        #[cfg(debug_assertions)]
        trace!(public_key = %item.public_key, "Encryption key details");

        let inserted = diesel::insert_into(encryption_keys::table)
            .values(&item)
            .returning(EncryptionKey::as_returning())
            .get_result(conn)?;
        info!(id = inserted.id, name = %inserted.name, "Encryption key created");
        Ok(inserted)
    }

    fn create_many(
        &self,
        conn: &mut SqliteConnection,
        items: Vec<NewEncryptionKey>,
    ) -> Result<Vec<EncryptionKey>> {
        use diesel::Connection;

        debug!(count = items.len(), "Creating multiple encryption keys");
        #[cfg(debug_assertions)]
        {
            for item in &items {
                trace!(name = %item.name, public_key = %item.public_key, "Creating encryption key with details");
            }
        }

        let result = conn.transaction(|conn| {
            let mut encryption_keys: Vec<EncryptionKey> = Vec::new();

            for item in items {
                let encryption_key = diesel::insert_into(encryption_keys::table)
                    .values(&item)
                    .returning(EncryptionKey::as_returning())
                    .get_result(conn)?;

                encryption_keys.push(encryption_key)
            }

            Ok(encryption_keys)
        });

        match &result {
            Ok(keys) => info!(count = keys.len(), "Encryption keys created"),
            Err(_) => warn!("Failed to create encryption keys"),
        }

        result.map_err(|error: diesel::result::Error| DatabaseError::from(error))
    }
}

impl RepositoryByName<EncryptionKey> for EncryptionKeyRepository {
    fn exists_by_name(&self, conn: &mut SqliteConnection, name: &str) -> Result<bool> {
        debug!(name, "Checking if encryption key exists by name");
        let result = encryption_keys::table
            .filter(encryption_keys::name.eq(name))
            .select(EncryptionKey::as_select())
            .first(conn)
            .optional()?;

        match result {
            None => {
                info!(
                    name,
                    exists = false,
                    "Encryption key existence check complete"
                );
                Ok(false)
            }
            Some(_) => {
                info!(
                    name,
                    exists = true,
                    "Encryption key existence check complete"
                );
                Ok(true)
            }
        }
    }
}

impl RepositoryGenericUpdate<EncryptionKey, UpdatedApiEncryptionKey> for EncryptionKeyRepository {
    fn update(
        &self,
        conn: &mut SqliteConnection,
        item: UpdatedApiEncryptionKey,
    ) -> Result<EncryptionKey> {
        debug!(id = item.id, "Updating encryption key");
        #[cfg(debug_assertions)]
        if let Some(ref name) = item.name {
            trace!(id = item.id, name = %name, "Encryption key update details");
        }

        let updated = diesel::update(encryption_keys::table)
            .filter(encryption_keys::id.eq(item.id))
            .set(&item)
            .returning(EncryptionKey::as_returning())
            .get_result(conn)?;
        info!(id = updated.id, name = %updated.name, "Encryption key updated");
        Ok(updated)
    }
}

#[allow(dead_code)]
fn fetch_secrets_for_encryption_key_id(
    conn: &mut SqliteConnection,
    encryption_key_id: i32,
) -> Vec<Secret> {
    secrets::table
        .filter(secrets::encryption_key_id.eq(encryption_key_id))
        .load(conn)
        .unwrap_or_else(|_| vec![])
}

#[allow(dead_code)]
fn fetch_secrets_for_encryption_keys(
    conn: &mut SqliteConnection,
    encryption_keys: &Vec<EncryptionKey>,
) -> Vec<Secret> {
    Secret::belonging_to(encryption_keys)
        .select(Secret::as_select())
        .load(conn)
        .unwrap_or_else(|_| vec![])
}

#[allow(dead_code)]
fn group_secrets_per_encryption_key(
    secrets: Vec<Secret>,
    encryption_keys: &Vec<EncryptionKey>,
) -> Vec<EncryptionKeyWithSecrets> {
    secrets
        .grouped_by(encryption_keys)
        .into_iter()
        .zip(encryption_keys)
        .map(|(creds, encryption_key)| EncryptionKeyWithSecrets {
            encryption_key: encryption_key.clone(),
            secrets: creds,
        })
        .collect::<Vec<EncryptionKeyWithSecrets>>()
}
