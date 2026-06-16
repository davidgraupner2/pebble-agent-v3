use crate::errors::Result;
use crate::query::{DeleteQuery, FilterCondition, SortCondition};
use diesel::prelude::*;

pub trait RepositoryDynamicQuery<Entity> {
    /// Get records filtered and sorted by dynamic query parameters
    fn get_by_dynamic_query(
        &self,
        conn: &mut SqliteConnection,
        // query: &DynamicQuery,
        filters: &Vec<FilterCondition>,
        sort: Option<&Vec<SortCondition>>,
        page_size: i64,
        page_offset: i64,
        registry_id: String,
    ) -> Result<(Vec<Entity>, i64)>;

    fn delete_by_dynamic_query(
        &self,
        conn: &mut SqliteConnection,
        query: &DeleteQuery,
        registration_id: String,
    ) -> Result<usize>;
}
