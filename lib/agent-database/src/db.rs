use crate::errors::{DatabaseError, Result};
use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use diesel::r2d2::Pool;
use diesel_migrations::EmbeddedMigrations;
use diesel_migrations::MigrationHarness;
use diesel_migrations::embed_migrations;
use std::path::PathBuf;

#[cfg(feature = "tracing")]
use tracing::{error, info, warn};

// Embed migrations from the default "migrations" directory
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

pub type SqlitePool = Pool<ConnectionManager<SqliteConnection>>;

pub fn get_db_connection_pool(
    folder_name: &PathBuf,
    db_name: &str,
    max_connections: u32,
) -> Result<SqlitePool> {
    let db_file_name = folder_name.join(db_name).to_string_lossy().to_string();

    build_database(&db_file_name)?;

    let manager = ConnectionManager::<SqliteConnection>::new(db_file_name.clone());

    match Pool::builder()
        .max_size(max_connections)
        .test_on_check_out(true)
        .build(manager)
    {
        Ok(pool) => Ok(pool),
        Err(error) => Err(DatabaseError::PoolError(error.to_string())),
    }
}

pub fn build_database(db_name: &str) -> Result<()> {
    println!("Building Database");
    match SqliteConnection::establish(&db_name) {
        Ok(mut connection) => match connection.run_pending_migrations(MIGRATIONS) {
            Ok(migrated) => {
                #[cfg(feature = "tracing")]
                info!(database_migrations=%migrated.len(), "Database migrations executed successfully");

                Ok(())
            }
            Err(error) => {
                #[cfg(feature = "tracing")]
                warn!(errorMsg=%error,"Database migrations did NOT execute successfully!");

                println!("Error building database: {}", error);

                Err(DatabaseError::Migration(error.to_string()))
            }
        },
        Err(error) => {
            #[cfg(feature = "tracing")]
            error!(errorMsg=%error, database=%db_name, "Unable to connect to database");

            Err(DatabaseError::Connection(error))
        }
    }
}
