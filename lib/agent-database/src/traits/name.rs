use crate::errors::Result;
use diesel::prelude::*;

/// Specialized repository trait for name-based queries
/// # Type Parameters
/// * `Entity` - The database model type (e.g., from the schema)
pub trait RepositoryByName<Entity> {
    // Check if a record exists by its name
    fn exists_by_name(&self, conn: &mut SqliteConnection, name: &str) -> Result<bool>; //{
}
