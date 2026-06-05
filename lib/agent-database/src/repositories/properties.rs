use crate::DatabaseError;
use crate::errors::Result;
use crate::models::{Property, PropertyRecord, TypedProperty, UpdateProperty};
use crate::schema::properties;
use crate::traits::RepositoryGetSet;
use diesel::prelude::*;
use tracing::{debug, info, trace, warn};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct PropertyRepository;

impl PropertyRepository {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl RepositoryGetSet<TypedProperty, Property> for PropertyRepository {
    /// Get a specific record by name

    fn get(
        &self,
        conn: &mut SqliteConnection,
        name: String,
        agent_uuid: Option<String>,
    ) -> Result<Option<TypedProperty>> {
        let property = match agent_uuid {
            Some(uuid) => get_property_by_name_and_agent_uuid(conn, &name, uuid)?,
            None => get_property_by_name(conn, &name)?,
        };
        match property {
            Some(prop) => {
                trace!(name = %name, "Property retrieved");
                Ok(Some(convert_to_typed(prop)?))
            }
            None => {
                warn!(name = %name, "Property not found");
                Ok(None)
            }
        }
    }

    /// Set (update or create) a record
    fn set(&self, conn: &mut SqliteConnection, entity: Property) -> Result<TypedProperty> {
        if let Some(existing) = get_property_by_name(conn, &entity.name)? {
            if check_same(&existing, &entity) {
                return convert_to_typed(existing);
            } else {
                trace!(name = %entity.name, "Updating existing property");
                return Ok(update_property(conn, existing.id, entity)?);
            }
        } else {
            trace!(name = %entity.name, "Creating new property");
            return Ok(create_property(conn, entity)?);
        }
    }

    fn set_many(
        &self,
        conn: &mut SqliteConnection,
        entities: Vec<Property>,
    ) -> Result<Vec<TypedProperty>> {
        let count = entities.len();
        debug!(count, "Creating batch of property records");

        #[cfg(debug_assertions)]
        trace!(?entities, "Property Batch details");

        let result = conn.transaction(|conn| {
            let mut property_records: Vec<TypedProperty> = Vec::new();

            for (idx, item) in entities.into_iter().enumerate() {
                #[cfg(debug_assertions)]
                trace!(batch_index = idx, name = %item.name, "Inserting Property record");

                let inserted_property_record = diesel::insert_into(properties::table)
                    .values(&item)
                    .returning(PropertyRecord::as_returning())
                    .get_result(conn)
                    .map_err(|e| {
                        warn!(batch_index = idx, error = %e, "Failed to insert Property record in batch");
                        e
                    })?;

                property_records.push(convert_to_typed(inserted_property_record)?);
            }

            Ok(property_records)
        }).map_err(|error| {
            warn!(count, error = %error, "Batch Property creation failed");
            error
        });

        if let Ok(ref records) = result {
            info!(count = records.len(), "Successfully created Property batch");
        }
        result
    }

    /// Get a record OR Set (update or create) a record if that record does not exist
    fn get_or_set(&self, conn: &mut SqliteConnection, entity: Property) -> Result<TypedProperty> {
        if let Some(existing) = get_property_by_name(conn, &entity.name)? {
            return convert_to_typed(existing);
        } else {
            trace!(name = %entity.name, "Property does not exist, creating");
            Ok(create_property(conn, entity)?)
        }
    }

    /// Get all records
    fn get_all(
        &self,
        conn: &mut SqliteConnection,
        agent_uuid: Option<String>,
    ) -> Result<Vec<TypedProperty>> {
        let properties = match agent_uuid {
            Some(uuid) => get_all_for_agent(conn, uuid)?,
            None => get_all(conn)?,
        };

        // let properties = get_all_for_agent(conn,)?;
        trace!(count = properties.len(), "All properties retrieved");
        Ok(properties)
    }

    /// Delete a record by name
    fn delete(
        &self,
        conn: &mut SqliteConnection,
        name: String,
        agent_uuid: String,
    ) -> Result<usize> {
        match diesel::delete(
            properties::table
                .filter(properties::name.eq(&name))
                .filter(properties::agent_uuid.eq(&agent_uuid)),
        )
        .execute(conn)
        {
            Ok(size) => {
                info!(deleted = size, name = %name, "Property deleted");
                Ok(size)
            }
            Err(error) => {
                warn!(name = %name, "Failed to delete property");
                Err(error.into())
            }
        }
    }

    /// Delete all records
    fn delete_all(&self, conn: &mut SqliteConnection, agent_uuid: String) -> Result<usize> {
        match diesel::delete(properties::table.filter(properties::agent_uuid.eq(&agent_uuid)))
            .execute(conn)
        {
            Ok(size) => {
                info!(deleted = size, "All properties deleted");
                Ok(size)
            }
            Err(error) => {
                warn!("Failed to delete all properties");
                Err(error.into())
            }
        }
    }
}

/// Helper functions
/// ~~~~~~~~~~~~~~~~~~

/// Creates a new property
fn create_property(conn: &mut SqliteConnection, item: Property) -> Result<TypedProperty> {
    match diesel::insert_into(properties::table)
        .values(&item)
        .returning(PropertyRecord::as_returning())
        .get_result(conn)
    {
        Ok(property) => match property.to_typed() {
            Some(typed_property) => {
                info!(name = %item.name, "Property created");
                Ok(typed_property)
            }
            _ => {
                warn!(name = %item.name, "Failed to convert property to typed");
                Err(DatabaseError::ConversionError(
                    "Property".to_string(),
                    item.name,
                    "Typed".to_string(),
                ))
            }
        },
        Err(error) => {
            warn!(name = %item.name, "Failed to create property");
            Err(error.into())
        }
    }
}

/// Converts a PropertyRecord to a TypedProperty
fn convert_to_typed(property: PropertyRecord) -> Result<TypedProperty> {
    match property.to_typed() {
        Some(typed_property) => Ok(typed_property),
        _ => Err(DatabaseError::ConversionError(
            "Property".to_string(),
            property.name,
            "Typed".to_string(),
        )),
    }
}

/// Retrieves a property by name
fn get_property_by_name(conn: &mut SqliteConnection, name: &str) -> Result<Option<PropertyRecord>> {
    properties::table
        .filter(properties::name.eq(name))
        .select(PropertyRecord::as_select())
        .first(conn)
        .optional()
        .map_err(Into::into)
}

/// Retrieves a property by name and agent_uuid
fn get_property_by_name_and_agent_uuid(
    conn: &mut SqliteConnection,
    name: &str,
    agent_uuid: String,
) -> Result<Option<PropertyRecord>> {
    properties::table
        .filter(properties::name.eq(name))
        .filter(properties::agent_uuid.eq(agent_uuid))
        .select(PropertyRecord::as_select())
        .first(conn)
        .optional()
        .map_err(Into::into)
}

/// Updates an existing property
fn update_property(
    conn: &mut SqliteConnection,
    id: i32,
    update: Property,
) -> Result<TypedProperty> {
    let property_to_update = UpdateProperty {
        id,
        agent_uuid: update.agent_uuid,
        name: update.name,
        type_: update.type_,
        source: update.source,
        description: update.description,
        value_int: update.value_int,
        value_string: update.value_string,
        value_bool: update.value_bool,
        value_json: update.value_json,
    };

    match diesel::update(properties::table.filter(properties::id.eq(id)))
        .set(&property_to_update)
        .returning(PropertyRecord::as_returning())
        .get_result(conn)
    {
        Ok(property) => match property.to_typed() {
            Some(typed_property) => {
                info!(id = id, name = %property_to_update.name, "Property updated");
                Ok(typed_property)
            }
            _ => {
                warn!(id = id, "Failed to convert property to typed");
                Err(DatabaseError::ConversionError(
                    "Property".to_string(),
                    property_to_update.id.to_string(),
                    "Typed".to_string(),
                ))
            }
        },
        Err(error) => {
            warn!(id = id, "Failed to update property");
            Err(error.into())
        }
    }
}

/// Retrieves all properties
fn get_all(conn: &mut SqliteConnection) -> Result<Vec<TypedProperty>> {
    properties::table
        .select(PropertyRecord::as_select())
        .load(conn)
        .map_err(Into::into)
        .and_then(|records| {
            records
                .into_iter()
                .map(|prop| {
                    prop.to_typed().ok_or_else(|| {
                        DatabaseError::ConversionError(
                            "Property".to_string(),
                            prop.name,
                            "Typed".to_string(),
                        )
                        .into()
                    })
                })
                .collect()
        })
}

/// Retrieves all properties
fn get_all_for_agent(
    conn: &mut SqliteConnection,
    agent_uuid: String,
) -> Result<Vec<TypedProperty>> {
    properties::table
        .filter(properties::agent_uuid.eq(agent_uuid))
        .select(PropertyRecord::as_select())
        .load(conn)
        .map_err(Into::into)
        .and_then(|records| {
            records
                .into_iter()
                .map(|prop| {
                    prop.to_typed().ok_or_else(|| {
                        DatabaseError::ConversionError(
                            "Property".to_string(),
                            prop.name,
                            "Typed".to_string(),
                        )
                        .into()
                    })
                })
                .collect()
        })
}

/// Checks if two properties are the same
fn check_same(a: &PropertyRecord, b: &Property) -> bool {
    a.name == b.name
        && a.type_ == b.type_
        && a.source == b.source
        && a.description == b.description
        && a.value_int == b.value_int
        && a.value_string == b.value_string
        && a.value_bool == b.value_bool
        && a.value_json == b.value_json
}
