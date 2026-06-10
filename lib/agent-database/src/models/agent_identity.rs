use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::schema::agent_identities;

#[derive(Queryable, Selectable, Identifiable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = agent_identities)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct AgentIdentity {
    pub id: i32,
    pub registration_id: String,
    pub pubkey_fingerprint: String,
    pub pubkey_b64u: String,
    pub agent_id: String,
    pub status: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = agent_identities)]
pub struct NewAgentIdentity {
    pub registration_id: String,
    pub pubkey_fingerprint: String,
    pub pubkey_b64u: String,
    pub agent_id: String,
    pub status: String,
}

impl NewAgentIdentity {
    pub fn new(pubkey_fingerprint: String, pubkey_b64u: String, agent_id: String) -> Self {
        Self {
            registration_id: Uuid::new_v4().to_string(),
            pubkey_fingerprint,
            pubkey_b64u,
            agent_id,
            status: "active".to_string(),
        }
    }
}
