use crate::DatabaseError;
use crate::schema::registration_challenges;
use base64ct::{Base64UrlUnpadded, Encoding};
use chrono::NaiveDateTime;
use diesel::prelude::*;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[derive(Queryable, Selectable, Identifiable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = registration_challenges)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct AgentRegistrationChallenge {
    pub id: i32,
    pub challenge_id: String,
    pub nonce_b64u: String,
    pub pubkey_fingerprint_b64u: String,
    pub registration_id: String,
    pub created_at: NaiveDateTime,
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = registration_challenges)]
pub struct NewAgentRegistrationChallenge {
    pub challenge_id: String,
    pub nonce_b64u: String,
    pub pubkey_fingerprint_b64u: String,
    pub registration_id: String,
}

impl NewAgentRegistrationChallenge {
    pub fn new(pubkey: String, registration_id: String) -> Result<Self, DatabaseError> {
        // Generate a challenge id
        let challenge_id = Uuid::new_v4().to_string();

        // Generate a random nonce that is base64 encoded
        let mut nonce = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut nonce);
        let nonce_b64u = Base64UrlUnpadded::encode_string(&nonce);

        // Generate a base64 encoded fingerprint for the passed in public key
        let pk_bytes = Base64UrlUnpadded::decode_vec(&pubkey).map_err(|_| {
            DatabaseError::PublicKeyEncodingError("Invalid public key encoding".to_string())
        })?;
        let digest = Sha256::digest(&pk_bytes);
        let pubkey_fingerprint_b64u = Base64UrlUnpadded::encode_string(&digest);

        Ok(Self {
            challenge_id,
            nonce_b64u,
            pubkey_fingerprint_b64u,
            registration_id,
        })
    }
}
