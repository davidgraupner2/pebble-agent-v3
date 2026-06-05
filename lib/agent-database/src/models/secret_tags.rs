use crate::models::{Secret, Tags};
use crate::schema::secret_tags;
use diesel::prelude::*;

#[derive(Identifiable, Queryable, Associations, Debug, Selectable, Insertable)]
#[diesel(belongs_to(Secret, foreign_key = secret_id))]
#[diesel(belongs_to(Tags, foreign_key = tag_id))]
#[diesel(table_name = secret_tags)]
#[diesel(primary_key(secret_id, tag_id))]
pub struct SecretTag {
    pub secret_id: i32,
    pub tag_id: i32,
}
