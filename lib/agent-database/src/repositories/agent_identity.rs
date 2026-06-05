use crate::errors::Result;
use crate::models::{AgentIdentity, NewAgentIdentity};
use crate::schema::agent_identities::dsl::*;
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
            agent_uuid = %new.agent_uuid,
            fingerprint = %new.pubkey_fingerprint,
            "Creating agent identity"
        );

        let record = diesel::insert_into(agent_identities)
            .values(&new)
            .get_result(conn)?;

        info!(
            agent_uuid = %new.agent_uuid,
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

    pub fn get_by_uuid(
        &self,
        conn: &mut SqliteConnection,
        uuid: &str,
    ) -> Result<Option<AgentIdentity>> {
        let result = agent_identities
            .filter(agent_uuid.eq(uuid))
            .first::<AgentIdentity>(conn)
            .optional()?;

        Ok(result)
    }
}
