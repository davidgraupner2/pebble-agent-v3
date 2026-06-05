use crate::models::{Cache, Tags};
use crate::schema::cache_tags;
use diesel::prelude::*;

#[derive(Identifiable, Queryable, Associations, Debug, Selectable, Insertable)]
#[diesel(belongs_to(Cache, foreign_key = cache_id))]
#[diesel(belongs_to(Tags, foreign_key = tag_id))]
#[diesel(table_name = cache_tags)]
#[diesel(primary_key(cache_id, tag_id))]
pub struct CacheTag {
    pub cache_id: i32,
    pub tag_id: i32,
}
