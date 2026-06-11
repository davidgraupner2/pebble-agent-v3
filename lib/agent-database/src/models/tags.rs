use crate::schema::tags;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use salvo::oapi::ToSchema;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Selectable, Identifiable, PartialEq, Serialize, Debug, Clone, ToSchema)]
#[diesel(table_name = tags)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Tags {
    pub id: i32,
    pub name: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable, Deserialize, Clone)]
#[diesel(table_name = tags)]
pub struct NewTag {
    pub name: String,
}
