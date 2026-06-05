use crate::errors::Result;
use diesel::prelude::*;

/// Generic repository trait for Insert operations on database entities.
///
/// # Type Parameters
/// * `Entity` - The database model type (e.g., from the schema)
/// * `NewEntity` - The insertable type (typically a struct with `#[derive(Insertable)]`)
pub trait RepositoryGenericInsert<Entity, NewEntity> {
    /// Insert a single record into the database
    fn create(&self, conn: &mut SqliteConnection, item: NewEntity) -> Result<Entity>;

    /// Insert multiple records into the database in a single transaction
    fn create_many(
        &self,
        conn: &mut SqliteConnection,
        items: Vec<NewEntity>,
    ) -> Result<Vec<Entity>>;
}

/// Generic repository trait for Update operations on database entities.
///
/// # Type Parameters
/// * `Entity` - The database model type (e.g., from the schema)
/// * `UpdateEntity` - The updateable type for partial updates
pub trait RepositoryGenericUpdate<Entity, UpdateEntity> {
    /// Update a record by its primary key ID
    fn update(&self, conn: &mut SqliteConnection, update: UpdateEntity) -> Result<Entity>;
}
