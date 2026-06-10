use crate::errors::Result;
use diesel::prelude::*;

/// Specialized repository trait for property  bases sets and gets
/// # Type Parameters
/// * `Entity` - The database model type
/// * `NewUpdatedEntity` - The new or updated entity
pub trait RepositoryGetSet<Entity, NewUpdatedEntity> {
    /// Get a specific record by name
    fn get(
        &self,
        conn: &mut SqliteConnection,
        name: String,
        agent_uuid: String,
    ) -> Result<Option<Entity>>;

    /// Set a specific record
    fn set(&self, conn: &mut SqliteConnection, entity: NewUpdatedEntity) -> Result<Entity>;

    fn set_many(
        &self,
        conn: &mut SqliteConnection,
        entities: Vec<NewUpdatedEntity>,
    ) -> Result<Vec<Entity>>;

    /// Get a record if it exists, otherwise set it
    fn get_or_set(&self, conn: &mut SqliteConnection, entity: NewUpdatedEntity) -> Result<Entity>;

    /// Get all property records
    fn get_all(&self, conn: &mut SqliteConnection, agent_uuid: String) -> Result<Vec<Entity>>;

    /// Delete a record by name
    fn delete(
        &self,
        conn: &mut SqliteConnection,
        name: String,
        agent_uuid: String,
    ) -> Result<usize>;

    /// Delete all records
    fn delete_all(&self, conn: &mut SqliteConnection, agent_uuid: String) -> Result<usize>;
}
