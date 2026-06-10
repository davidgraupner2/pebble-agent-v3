pub mod api;
pub mod config;
pub mod error;
#[cfg(target_os = "linux")]
pub mod linux;
pub mod properties;
pub mod server_core;
pub mod state;
#[cfg(windows)]
pub mod windows;

use crate::config::Config;
use crate::error::{ApiError, Result};
use crate::properties::*;
use agent_core::prelude::*;
use agent_database::{get_db_connection_pool, PropertyValue, RepositoryContainer};
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::SqliteConnection;
use std::sync::OnceLock;
use tracing_appender::non_blocking::WorkerGuard;

pub const SERVICE_NAME: &str = "PebbleAgentApiServer";
pub const SERVICE_DISPLAY_NAME: &str = "Pebble Agent API Server";
pub const SERVICE_DESCRIPTION: &str = "This API server is part of the Pebble Agent Suite and serves to a provide a persisted multi-tenanted database and API layer for the Pebble Agents that power your automation strategy";

/// Default property definitions
pub struct DefaultProperty {
    pub name: &'static str,
    pub value: PropertyValue,
    pub description: Option<&'static str>,
}

#[derive(Debug)]
pub struct BootstrapParameters {
    pub db_pool: Pool<ConnectionManager<SqliteConnection>>,
    pub repository_container: RepositoryContainer,
    pub config: Config,
    pub logging_format: String,
    pub logging_output: String,
    pub logging_level: String,
    pub port: i32,
}

// Define a static OnceLock to hold the logging worker guards
// We need to store these to ensure logging continues for as long as the application is running
pub static LOGGING_WORKER_GUARDS: OnceLock<Vec<WorkerGuard>> = OnceLock::new();

// Define a static OnceLock to hold the repository traits
// It is uninitialized by default
// static REPOSITORIES: OnceLock<RepositoryContainer> = OnceLock::new();

// Define a static Oncelock to hold the config object
// - this config object is leveraged to get and set properties in the database
// static CONFIG: OnceLock<Config> = OnceLock::new();

// Bootstrap the API Server
// - This sets up all prerequisite items such as the database
pub fn bootstrap_api_server() -> Result<BootstrapParameters> {
    // Initialize the global database pool
    let pool = initialise_database_pool()?;

    // Initialize repository container
    // - This house all the database traits we use to access the database
    let repository_container = RepositoryContainer::initialize();

    // Initialize the global config object
    // - This allows us to easily get and set database properties
    let config = initialise_global_config(&pool, &repository_container)?;

    // Initalise and set default api properties if they haven't been set yet
    initialise_database_properties(&config)?;

    // Gets the API Port we will be using
    let port = config.get_int(
        PROPERTY_API_PORT,
        DEFAULT_PROPERTY_API_PORT,
        RuntimeConstants::global().api_id().to_string(),
    );

    // Get the logging properties
    let logging_properties = get_logging_properties(&config)?;

    Ok(BootstrapParameters {
        db_pool: pool,
        repository_container: repository_container,
        config: config,
        logging_format: logging_properties.0,
        logging_level: logging_properties.1,
        logging_output: logging_properties.2,
        port: port,
    })
}

/// Initialize the global database pool
///
/// Under the hood this leverages the agent_database crate that creates the database - if needed
pub(crate) fn initialise_database_pool() -> Result<Pool<ConnectionManager<SqliteConnection>>> {
    match get_db_connection_pool(
        RuntimeConstants::global().folders().supplementary_files(),
        DATABASE_NAME,
        10,
    ) {
        Ok(pool) => Ok(pool),
        Err(error) => Err(ApiError::DatabaseError(error.to_string())),
    }
}
/// Initialize the global configuration
///
/// Retrieves the database pool and property repository to create a Config instance.
/// Returns an error if the pool or repository are not initialized.
pub(crate) fn initialise_global_config(
    pool: &Pool<ConnectionManager<SqliteConnection>>,
    repositories: &RepositoryContainer,
) -> Result<Config> {
    // let pool = match DB_POOL.get() {
    //     Some(pool) => pool,
    //     None => {
    //         return Err(ApiError::ConfigError(
    //             "Unable to initialise global config - Database pool could not be obtained"
    //                 .to_string(),
    //         ));
    //     }
    // };

    let property_repository = repositories.properties_repo.clone();

    // let property_repository = match REPOSITORIES.get() {
    //     Some(repository_container) => repository_container.properties_repo.clone(),
    //     None => {
    //         return Err(ApiError::ConfigError(
    //             "Unable to initialise global config - Property repository could not be obtained"
    //                 .to_string(),
    //         ));
    //     }
    // };

    // let connection_string_repository = match REPOSITORIES.get() {
    //     Some(repository_container) => repository_container.connection_string_repo.clone(),
    //     None => {
    //         return Err(ApiError::ConfigError(
    //             "Unable to initialise global config - Connection String repository could not be obtained"
    //                 .to_string(),
    //         ));
    //     }
    // };

    // let events_repository = match REPOSITORIES.get() {
    //     Some(repository_container) => repository_container.events_repo.clone(),
    //     None => {
    //         return Err(ApiError::ConfigError(
    //             "Unable to initialise global config - Events repository could not be obtained"
    //                 .to_string(),
    //         ));
    //     }
    // };

    Ok(Config {
        db_pool: pool.clone(),
        property_repo: property_repository,
        // connection_string_repo: connection_string_repository,
        // event_repo: events_repository,
    })
}

/// Initialize default database properties
///
/// Sets up default property values in the database. Creates properties if they don't exist.
pub(crate) fn initialise_database_properties(config: &Config) -> Result<()> {
    let api_id = RuntimeConstants::global().api_id();

    // match CONFIG.get() {
    //     Some(config) => {
    for prop in default_properties() {
        match prop.value {
            PropertyValue::Int(val) => {
                let _ =
                    config.get_or_set_int(prop.name, val, prop.description, api_id.to_string())?;
            }
            PropertyValue::String(ref val) => {
                let _ = config.get_or_set_string(
                    prop.name,
                    val,
                    prop.description,
                    api_id.to_string(),
                )?;
            }
            PropertyValue::Bool(val) => {
                let _ =
                    config.get_or_set_bool(prop.name, val, prop.description, api_id.to_string())?;
            }
            PropertyValue::Json(ref val) => {
                let _ = config.get_or_set_json(
                    prop.name,
                    val.clone(),
                    prop.description,
                    api_id.to_string(),
                )?;
            }
        }
    }
    Ok(())
    // }
    // None => Err(ApiError::ConfigError(
    //     "Error accessing global config to initialise global database properties".to_string(),
    // )),
    // }
}

/// Database Property defaults
fn default_properties() -> Vec<DefaultProperty> {
    vec![
        DefaultProperty {
            name: PROPERTY_API_PORT,
            value: PropertyValue::Int(DEFAULT_PROPERTY_API_PORT),
            description: Some("TCP Port used by the API server as the listening port."),
        },
        DefaultProperty {
            name: PROPERTY_API_LOGGING_LEVEL,
            value: PropertyValue::String("error".to_string()),
            description: Some("Logging level to use - valid values include  ['info','warning','error','debug','trace']"),
        },
        DefaultProperty {
            name: PROPERTY_API_LOGGING_FORMAT,
            value: PropertyValue::String("json".to_string()),
            description: Some("Logging format to use - valid values include ['json','full','pretty','compact']"),
        },
        DefaultProperty {
            name: PROPERTY_API_LOGGING_OUTPUT,
            value: PropertyValue::String("file".to_string()),
            description: Some("Logging output to use - valid values include  ['file','console','both']"),
        },
        DefaultProperty {
            name: PROPERTY_API_CS_VALIDATE,
            value: PropertyValue::Bool(true),
            description: Some("Indicates whether we should validate the connection string before persisting to storage. If validation fails then the connection string will be rejected."),
        },
        DefaultProperty {
            name: PROPERTY_API_CS_VALIDATE_AUDIENCE,
            value: PropertyValue::String("thepebble.io".to_string()),
            description: Some("If configured, then this value is used to validate the audience configured in the connection string. If the audience value configured here does not match the connection string audience, then the connection string will be rejected"),
        },
        DefaultProperty {
            name: PROPERTY_API_CS_VALIDATE_SCHEME,
            value: PropertyValue::String("wss".to_string()),
            description: Some("Indicates the required URL scheme for connection string validation. If the scheme of a newly provided connection string does not match this value, the connection string will be rejected"),
        },
        DefaultProperty {
            name: PROPERTY_API_CS_VALIDATE_SECRET_PARAMETER_NAME,
            value: PropertyValue::String("s".to_string()),
            description: Some("Indicates the parameter name for the secret provided as part of the connection string. The lack of this parameter in the querystring will result in a new connection string being rejected during validation"),
        },
        DefaultProperty {
            name: PROPERTY_API_CS_EXTRACT_ENVIRONMENT,
            value: PropertyValue::String("sub:.*:0".to_string()),
            description: Some("Indicates how to extract the environment from the Connection String JWT/Secret claims. Seperated by ':' the first parameter depicts the claim name, the second depicts the REGEX extractor to use and the third parameter depicts the REGEX group to return as the environment"),
        },
        DefaultProperty {
            name: PROPERTY_API_CS_EXTRACT_ENVIRONMENT,
            value: PropertyValue::String("sub:.*:0".to_string()),
            description: Some("Indicates how to extract the environment from the Connection String JWT/Secret claims. Seperated by ':' the first parameter depicts the claim name, the second depicts the REGEX extractor to use and the third parameter depicts the REGEX group to return as the environment"),
        },
        DefaultProperty {
            name: PROPERTY_API_JWT_EXPIRY_MINUTES,
            value: PropertyValue::Int(DEFAULT_PROPERTY_API_JWT_EXPIRY_MINUTES),
            description: Some("Indicates how many minutes a JWT, issued by the API will be valid for. A value of 0 allows the JWT to never expire"),
        },
        DefaultProperty {
            name: PROPERTY_API_AGENT_REGISTRATION_EXPIRY_SECS,
            value: PropertyValue::Int(DEFAULT_PROPERTY_API_AGENT_REGISTRATION_EXPIRY_SECS),
            description: Some("Indicates how many seconds to allow for an agent registration process to complete"),
        },
    ]
}

/// Retrieve logging configuration properties
///
/// Returns a tuple of (logging_format, logging_level, logging_output) from the database,
/// with sensible defaults if properties are not set.
pub(crate) fn get_logging_properties(config: &Config) -> Result<(String, String, String)> {
    let api_id = RuntimeConstants::global().api_id();
    // match CONFIG.get() {
    //     Some(config) => {
    let logging_format = config.get_string(PROPERTY_API_LOGGING_FORMAT, "json", api_id.to_string());
    let logging_level = config.get_string(PROPERTY_API_LOGGING_LEVEL, "error", api_id.to_string());
    let logging_output = config.get_string(PROPERTY_API_LOGGING_OUTPUT, "file", api_id.to_string());
    Ok((logging_format, logging_level, logging_output))
    //     }
    //     None => Err(ApiError::ConfigError(
    //         "Error accessing global config to get logging properties".to_string(),
    //     )),
    // }
}
