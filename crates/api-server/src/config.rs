use crate::error::{ApiError, Result};
use agent_database::{
    query::{SortCondition, SortDirection},
    ConnectionStringRepository, EventRepository, Property, PropertyRepository, PropertyValue,
    RepositoryDynamicQuery, RepositoryGetSet,
};
use diesel::{
    r2d2::{ConnectionManager, Pool, PooledConnection},
    SqliteConnection,
};
use tracing::error;

#[derive(Clone, Debug)]
pub struct Config {
    pub db_pool: Pool<ConnectionManager<SqliteConnection>>,
    pub property_repo: PropertyRepository,
    // pub connection_string_repo: ConnectionStringRepository,
    // pub event_repo: EventRepository,
}

impl Config {
    fn db_connection(&self) -> Result<PooledConnection<ConnectionManager<SqliteConnection>>> {
        match self.db_pool.get() {
            Ok(connection) => Ok(connection),
            Err(error) => Err(ApiError::DataAccessError(error.to_string())),
        }
    }

    // ==================== GET Connection String Operations ====================
    // pub fn get_connection_strings(&self) -> Vec<AgentConnectionString> {
    //     let filters = vec![];
    //     let mut sort_conditions: Vec<SortCondition> = vec![];
    //     sort_conditions.push(SortCondition {
    //         field: "status".to_string(),
    //         direction: SortDirection::Desc,
    //     });

    //     let mut agent_connection_strings = vec![];

    //     match self.db_pool.get() {
    //         Ok(mut conn) => {
    //             match self.connection_string_repo.get_by_dynamic_query(
    //                 &mut conn,
    //                 &filters,
    //                 Some(&sort_conditions),
    //                 2,
    //                 1,
    //             ) {
    //                 Ok((connection_strings, _count)) => {
    //                     for connection_string in connection_strings.into_iter() {
    //                         agent_connection_strings.push(AgentConnectionString::new(
    //                             self.db_pool.clone(),
    //                             self.connection_string_repo.clone(),
    //                             connection_string,
    //                         ));
    //                     }
    //                 }
    //                 Err(error) => {
    //                     error!(errorMsg=%error, "Unable to retrieve connection string from database");
    //                 }
    //             }
    //         }
    //         Err(error) => {
    //             error!(errorMsg=%error,
    //                 "Unable to obtain database connection to get connection strings",
    //             );
    //         }
    //     }
    //     agent_connection_strings
    // }

    // ==================== GET Operations ====================

    /// Get an integer property with a default fallback
    #[allow(dead_code)]
    pub fn get_int(&self, name: &str, default: i32, registration_id: String) -> i32 {
        match self.db_connection() {
            Ok(mut conn) => {
                match self
                    .property_repo
                    .get(&mut conn, name.to_string(), registration_id)
                {
                    Ok(Some(typed_prop)) => {
                        if let PropertyValue::Int(val) = typed_prop.value {
                            return val;
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        default
    }

    /// Get a string property with a default fallback
    #[allow(dead_code)]
    pub fn get_string(&self, name: &str, default: &str, registration_id: String) -> String {
        match self.db_connection() {
            Ok(mut conn) => {
                match self
                    .property_repo
                    .get(&mut conn, name.to_string(), registration_id)
                {
                    Ok(Some(typed_prop)) => {
                        if let PropertyValue::String(val) = typed_prop.value {
                            return val;
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        default.to_string()
    }

    /// Get a boolean property with a default fallback
    #[allow(dead_code)]
    pub fn get_bool(&self, name: &str, default: bool, registration_id: String) -> bool {
        match self.db_connection() {
            Ok(mut conn) => {
                match self
                    .property_repo
                    .get(&mut conn, name.to_string(), registration_id)
                {
                    Ok(Some(typed_prop)) => {
                        if let PropertyValue::Bool(val) = typed_prop.value {
                            return val;
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        default
    }

    /// Get a JSON property with a default fallback
    #[allow(dead_code)]
    pub fn get_json(
        &self,
        name: &str,
        default: serde_json::Value,
        registration_id: String,
    ) -> serde_json::Value {
        match self.db_connection() {
            Ok(mut conn) => {
                match self
                    .property_repo
                    .get(&mut conn, name.to_string(), registration_id)
                {
                    Ok(Some(typed_prop)) => {
                        if let PropertyValue::Json(val) = typed_prop.value {
                            return val;
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        default
    }

    // ==================== SET Operations ====================

    /// Set an integer property
    pub fn get_or_set_int(
        &self,
        name: &str,
        value: i32,
        description: Option<&str>,
        registration_id: String,
    ) -> Result<i32> {
        let mut conn = self.db_connection()?;
        let property = Property {
            name: name.to_string(),
            type_: "int".to_string(),
            description: description.map(|d| d.to_string()),
            source: "config".to_string(),
            value_int: Some(value),
            value_string: None,
            value_bool: None,
            value_json: None,
            registration_id: registration_id,
        };
        let result = self.property_repo.get_or_set(&mut conn, property)?;
        if let PropertyValue::Int(val) = result.value {
            Ok(val)
        } else {
            Err(ApiError::DataAccessError(
                "Expected integer property value".to_string(),
            ))
        }
    }

    /// Set a string property
    pub fn get_or_set_string(
        &self,
        name: &str,
        value: &str,
        description: Option<&str>,
        registration_id: String,
    ) -> Result<String> {
        let mut conn = self.db_connection()?;
        let property = Property {
            name: name.to_string(),
            type_: "string".to_string(),
            description: description.map(|d| d.to_string()),
            source: "config".to_string(),
            value_int: None,
            value_string: Some(value.to_string()),
            value_bool: None,
            value_json: None,
            registration_id: registration_id,
        };
        let result = self.property_repo.get_or_set(&mut conn, property)?;
        if let PropertyValue::String(val) = result.value {
            Ok(val)
        } else {
            Err(ApiError::DataAccessError(
                "Expected string property value".to_string(),
            ))
        }
    }

    /// Set a boolean property
    pub fn get_or_set_bool(
        &self,
        name: &str,
        value: bool,
        description: Option<&str>,
        registration_id: String,
    ) -> Result<bool> {
        let mut conn = self.db_connection()?;
        let property = Property {
            name: name.to_string(),
            type_: "bool".to_string(),
            description: description.map(|d| d.to_string()),
            source: "config".to_string(),
            value_int: None,
            value_string: None,
            value_bool: Some(if value { 1 } else { 0 }),
            value_json: None,
            registration_id: registration_id,
        };
        let result = self.property_repo.get_or_set(&mut conn, property)?;
        if let PropertyValue::Bool(val) = result.value {
            Ok(val)
        } else {
            Err(ApiError::DataAccessError(
                "Expected bool property value".to_string(),
            ))
        }
    }

    /// Set a JSON property
    pub fn get_or_set_json(
        &self,
        name: &str,
        value: serde_json::Value,
        description: Option<&str>,
        registration_id: String,
    ) -> Result<serde_json::Value> {
        let mut conn = self.db_connection()?;
        let property = Property {
            name: name.to_string(),
            type_: "json".to_string(),
            description: description.map(|d| d.to_string()),
            source: "config".to_string(),
            value_int: None,
            value_string: None,
            value_bool: None,
            value_json: Some(serde_json::to_string(&value)?),
            registration_id: registration_id,
        };
        let result = self.property_repo.get_or_set(&mut conn, property)?;
        if let PropertyValue::Json(val) = result.value {
            Ok(val)
        } else {
            Err(ApiError::DataAccessError(
                "Expected bool property value".to_string(),
            ))
        }
    }
}
