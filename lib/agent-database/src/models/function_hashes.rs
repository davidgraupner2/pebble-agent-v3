use crate::schema::function_hashes;
use crate::source_default;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Selectable, Serialize, Debug)]
#[diesel(table_name = function_hashes)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct FunctionHash {
    pub id: i32,
    pub function_hash: String,
    pub description: Option<String>,
    pub source: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable, Deserialize, Clone)]
#[diesel(table_name = function_hashes)]
pub struct NewFunctionHash {
    pub function_hash: String,
    #[serde(default = "source_default")]
    pub source: Option<String>,
    pub description: Option<String>,
}

#[derive(Insertable, Debug, AsChangeset, Identifiable, Deserialize)]
#[diesel(table_name = function_hashes)]
pub struct UpdateFunctionHash {
    pub id: i32,
    pub function_hash: String,
    #[serde(default = "source_default")]
    pub source: Option<String>,
    pub description: Option<String>,
}
