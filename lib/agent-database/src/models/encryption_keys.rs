use crate::{models::Secret, schema::encryption_keys};
use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Identifiable, Selectable, Serialize, Debug, PartialEq, Clone)]
#[diesel(table_name = encryption_keys)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct EncryptionKey {
    pub id: i32,
    pub name: String,
    #[serde(skip, default)]
    pub public_key: String,
    pub enabled: i32,
    pub source: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Serialize, Debug, Clone)]
pub struct EncryptionKeyWithSecrets {
    #[serde(flatten)]
    pub encryption_key: EncryptionKey,
    pub secrets: Vec<Secret>,
}

#[derive(Insertable, Deserialize, Clone, Debug)]
#[diesel(table_name = encryption_keys)]
pub struct NewEncryptionKey {
    pub name: String,
    pub public_key: String,
    pub source: String,
}

#[derive(Deserialize, Serialize)]
pub struct ApiEncryptionKey {
    pub name: String,
}

#[derive(Queryable, Identifiable, AsChangeset, Deserialize, Clone)]
#[diesel(table_name = encryption_keys)]
pub struct UpdatedApiEncryptionKey {
    pub id: i32,
    pub name: Option<String>,
    pub enabled: Option<i32>,
}

#[derive(Deserialize, Serialize)]
pub struct DecryptionKey {
    pub id: i32,
    pub name: String,
    pub private_key: String,
    pub notes: String,
}
