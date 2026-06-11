use crate::SecureAgentIdentity;
use crate::errors::Result;
use crate::models::{AgentIdentity, NewAgentIdentity};
use crate::schema::agent_identities::dsl::*;
use diesel::associations::HasTable;
use diesel::dsl::sql;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use tracing::{debug, info};

#[derive(Debug, Clone)]
pub struct AgentIdentityRepository;

impl AgentIdentityRepository {
    pub(crate) fn new() -> Self {
        Self {}
    }

    pub fn create(
        &self,
        conn: &mut SqliteConnection,
        new: NewAgentIdentity,
    ) -> Result<AgentIdentity> {
        debug!(
            registration_id = %new.registration_id,
            fingerprint = %new.pubkey_fingerprint,
            "Creating agent identity"
        );

        let record = diesel::insert_into(agent_identities)
            .values(&new)
            .get_result(conn)?;

        info!(
            registration_id = %new.registration_id,
            "Agent identity created"
        );

        Ok(record)
    }

    pub fn get_by_fingerprint(
        &self,
        conn: &mut SqliteConnection,
        fingerprint: &str,
    ) -> Result<Option<AgentIdentity>> {
        let result = agent_identities
            .filter(pubkey_fingerprint.eq(fingerprint))
            .first::<AgentIdentity>(conn)
            .optional()?;

        Ok(result)
    }

    pub fn get_by_registration_id(
        &self,
        conn: &mut SqliteConnection,
        uuid: &str,
    ) -> Result<Option<AgentIdentity>> {
        let result = agent_identities
            .filter(registration_id.eq(uuid))
            .first::<AgentIdentity>(conn)
            .optional()?;

        Ok(result)
    }

    pub fn get_all(&self, conn: &mut SqliteConnection) -> Result<Vec<SecureAgentIdentity>> {
        let result = agent_identities
            .select(SecureAgentIdentity::as_select())
            .load::<SecureAgentIdentity>(conn)?;

        Ok(result)
    }

    pub fn get_count(&self, conn: &mut SqliteConnection) -> Result<i64> {
        let count = agent_identities::table().count().get_result(conn)?;
        Ok(count)
    }
}
