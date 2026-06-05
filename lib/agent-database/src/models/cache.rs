use crate::models::connection_strings::serialize_tag_names;
use crate::source_default;
use crate::{models::Tags, schema::cache};
use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

// ============= RESPONSE TYPES =============

#[derive(Serialize, Debug, Clone)]
pub struct CacheWithTags {
    pub cache: Cache,
    #[serde(serialize_with = "serialize_tag_names")]
    pub tags: Vec<Tags>,
}

// ============= DATABASE/QUERYABLE TYPES =============

#[derive(Queryable, Selectable, Identifiable, PartialEq, Serialize, Debug, Clone)]
#[diesel(table_name = cache)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Cache {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub type_: String,
    pub value: String,
    pub source: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub expires_at: Option<NaiveDateTime>,
}

// ============= REQUEST/INSERTION TYPES =============

#[derive(Insertable, Deserialize, Clone, Debug)]
#[diesel(table_name = cache)]
pub struct NewCache {
    pub name: String,
    pub description: Option<String>,
    pub type_: String,
    pub value: String,
    pub source: String,
    pub expires_at: Option<NaiveDateTime>,
}

#[derive(Deserialize, Clone)]
pub struct CreateCacheRequest {
    pub name: String,
    pub description: Option<String>,
    pub type_: String,
    pub value: String,
    #[serde(default = "source_default")]
    pub source: Option<String>,
    pub expires_at: Option<NaiveDateTime>,
    pub tags: Option<Vec<String>>,
}

#[derive(Insertable, Debug, AsChangeset, Identifiable, Deserialize)]
#[diesel(table_name = cache)]
pub struct UpdateCache {
    pub id: i32,
    pub name: Option<String>,
    pub description: Option<String>,
    pub type_: Option<String>,
    pub value: Option<String>,
    pub source: Option<String>,
    pub expires_at: Option<NaiveDateTime>,
}

#[derive(Deserialize, Clone)]
pub struct UpdateCacheRequest {
    pub id: i32,
    pub name: Option<String>,
    pub description: Option<String>,
    pub type_: Option<String>,
    pub value: Option<String>,
    #[serde(default = "source_default")]
    pub source: Option<String>,
    pub expires_at: Option<NaiveDateTime>,
    pub tags: Option<Vec<String>>,
}
