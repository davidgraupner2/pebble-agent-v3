use std::collections::HashMap;

use crate::{Tags, errors::Result};
use diesel::prelude::*;

/// Specialized repository trait for tag-based filtering
/// Useful for entities with related tags
/// # Type Parameters
/// * `Entity` - The database model type
/// * `Status` - The status enum type
pub trait RepositoryByTags<Entity> {
    fn get_tags_for(&self, conn: &mut SqliteConnection, entity: &Entity) -> Vec<Tags>;
    fn get_tags_for_many(
        &self,
        conn: &mut SqliteConnection,
        entities: &[Entity],
    ) -> Result<HashMap<i32, Vec<Tags>>>;
    fn create_tags_for(
        &self,
        conn: &mut SqliteConnection,
        entity: &Entity,
        tags: Vec<String>,
    ) -> Result<()>;
    fn delete_tags_for(&self, conn: &mut SqliteConnection, entity: &Entity) -> Result<usize>;
}
