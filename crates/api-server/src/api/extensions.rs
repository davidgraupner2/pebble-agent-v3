use agent_database::RepositoryContainer;
use diesel::r2d2::{ConnectionManager, PooledConnection};
use diesel::SqliteConnection;
use salvo::prelude::*;

use crate::config::Config;

type DbPool = diesel::r2d2::Pool<ConnectionManager<SqliteConnection>>;
type DbConn = PooledConnection<ConnectionManager<SqliteConnection>>;

// Define an extension trait for Salvo's depot
pub trait DepotExt {
    fn db_conn(&self) -> Result<DbConn, StatusError>;
    fn repositories(&self) -> Result<RepositoryContainer, StatusError>;
    fn config(&self) -> Result<Config, StatusError>;
}

// Implement the DepotExt trait for Salvo's Depot
// Allows us to cleanly access the db pool and other objects from the Depot Store
impl DepotExt for Depot {
    fn db_conn(&self) -> Result<DbConn, StatusError> {
        self.obtain::<DbPool>()
            .map_err(|_| {
                tracing::error!("Database pool missing from Depot");
                StatusError::internal_server_error().brief("Database pool configuration error.")
            })?
            .get()
            .map_err(|e| {
                tracing::error!("Failed to get database connection from pool: {}", e);
                StatusError::internal_server_error().brief("Database connection timeout.")
            })
    }
    fn repositories(&self) -> Result<RepositoryContainer, StatusError> {
        self.obtain::<RepositoryContainer>()
            .map_err(|_| {
                tracing::error!("Repository Container missing from Depot");
                StatusError::internal_server_error().brief("Repository configuration error.")
            })
            .map(|repo| repo.clone())
    }
    fn config(&self) -> Result<Config, StatusError> {
        self.obtain::<Config>()
            .map_err(|_| {
                tracing::error!("Config object missing from Depot");
                StatusError::internal_server_error().brief("Config error.")
            })
            .map(|repo| repo.clone())
    }
}
