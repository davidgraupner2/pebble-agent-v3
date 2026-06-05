use crate::models::connection_strings::serialize_tag_names;
use crate::models::secret_types::SecretValue;
use crate::{
    models::{EncryptionKey, Tags},
    schema::secrets,
    source_default,
};
use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Debug)]
#[serde(untagged)]
pub enum SecretValueOrEncrypted {
    Decrypted(SecretValue),
    Encrypted(String),
}

// ============= RESPONSE TYPES =============

#[derive(Serialize, Debug)]
pub struct SecretWithTags {
    pub secret: Secret,
    #[serde(serialize_with = "serialize_tag_names")]
    pub tags: Vec<Tags>,
}

#[derive(Serialize, Debug)]
pub struct DecryptedSecretWithTags {
    pub secret: DecryptedSecret,
    #[serde(serialize_with = "serialize_tag_names")]
    pub tags: Vec<Tags>,
}

// ============= DATABASE/QUERYABLE TYPES =============

#[derive(Queryable, Selectable, Identifiable, Serialize, Debug, PartialEq, Clone, Associations)]
#[diesel(table_name = secrets)]
#[diesel(belongs_to(EncryptionKey))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Secret {
    pub id: i32,
    pub name: String,
    pub secret_type: String,
    pub description: Option<String>,
    pub value: String, // Encrypted JSON string
    pub source: String,
    #[serde(skip, default)]
    pub ephemeral_key: Option<String>,
    #[serde(skip, default)]
    pub nonce: Option<String>,
    pub encryption_key_id: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Serialize, Debug)]
pub struct DecryptedSecret {
    pub id: i32,
    pub name: String,
    pub secret_type: String,
    pub description: Option<String>,
    pub value: SecretValueOrEncrypted, // Decrypted and deserialized
    pub source: String,
    pub encryption_key_id: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

// ============= REQUEST/INSERTION TYPES =============

#[derive(Insertable, Clone, Debug)]
#[diesel(table_name = secrets)]
pub struct NewSecret {
    pub name: String,
    pub secret_type: String,
    pub description: Option<String>,
    pub value: String, // Will be encrypted JSON
    pub source: String,
    pub encryption_key_id: i32,
    pub ephemeral_key: Option<String>,
    pub nonce: Option<String>,
}

#[derive(Deserialize, Clone)]
pub struct CreateSecretRequest {
    pub name: String,
    pub description: Option<String>,
    pub encryption_key_id: i32,
    #[serde(default = "source_default")]
    pub source: Option<String>,
    #[serde(flatten)]
    pub value: SecretValue,
    pub tags: Option<Vec<String>>,
}

#[derive(Queryable, Identifiable, AsChangeset, Deserialize, Clone)]
#[diesel(table_name = secrets)]
pub struct UpdateSecret {
    pub id: i32,
    pub name: Option<String>,
    pub secret_type: String,
    pub description: Option<String>,
    pub value: String,
    pub source: Option<String>,
    pub encryption_key_id: Option<i32>,
    pub ephemeral_key: Option<String>,
    pub nonce: Option<String>,
}

#[derive(Deserialize, Clone)]
pub struct UpdateSecretRequest {
    pub id: i32,
    pub name: Option<String>,
    pub description: Option<String>,
    #[serde(default = "source_default")]
    pub source: Option<String>,
    #[serde(flatten)]
    pub value: Option<SecretValue>,
    pub encryption_key_id: Option<i32>,
    pub tags: Option<Vec<String>>,
}
