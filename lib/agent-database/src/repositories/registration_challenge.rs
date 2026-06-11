use crate::errors::Result;
use crate::models::{AgentRegistrationChallenge, NewAgentRegistrationChallenge};
use crate::schema::registration_challenges::dsl::*;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use tracing::{debug, info, warn};

#[derive(Debug, Clone)]
pub struct RegistrationChallengeRepository;

impl RegistrationChallengeRepository {
    pub(crate) fn new() -> Self {
        Self {}
    }

    pub fn create(
        &self,
        conn: &mut SqliteConnection,
        new: NewAgentRegistrationChallenge,
    ) -> Result<AgentRegistrationChallenge> {
        debug!(
            agent_id = %new.registration_id,
            challenge_id = %new.challenge_id,
            "Creating agent agent registration challenge"
        );

        let record = diesel::insert_into(registration_challenges)
            .values(&new)
            .get_result(conn)?;

        info!(
            registration_id = %new.registration_id,
            "Agent registration challenge created"
        );

        Ok(record)
    }

    pub fn get_by_challenge_id(
        &self,
        conn: &mut SqliteConnection,
        uuid: &str,
    ) -> Result<Option<AgentRegistrationChallenge>> {
        let result = registration_challenges
            .filter(challenge_id.eq(uuid))
            .first::<AgentRegistrationChallenge>(conn)
            .optional()?;

        Ok(result)
    }

    /// Delete a record by name
    pub fn delete_by_challenge_id(&self, conn: &mut SqliteConnection, uuid: &str) -> Result<usize> {
        match diesel::delete(registration_challenges.filter(challenge_id.eq(uuid))).execute(conn) {
            Ok(size) => {
                info!(deleted = size, challenge_id = %uuid, "Registration Challenge deleted");
                Ok(size)
            }
            Err(error) => {
                warn!(challenge_id = %uuid, "Failed to delete Registration Challenge");
                Err(error.into())
            }
        }
    }
}
