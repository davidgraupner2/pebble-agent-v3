use crate::errors::Result;
use crate::schema::agent_jwt::dsl::*;
use crate::{AgentJwt, AgentJwtStatus, NewAgentJwt};
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use tracing::{debug, info, warn};

#[derive(Debug, Clone)]
pub struct AgentJwtRepository;

impl AgentJwtRepository {
    pub(crate) fn new() -> Self {
        Self {}
    }

    pub fn create(&self, conn: &mut SqliteConnection, new: NewAgentJwt) -> Result<AgentJwt> {
        debug!(
            registration_id = %new.registration_id,
            jti = %new.jti,
            "Creating agent jwt record"
        );

        let record: AgentJwt = diesel::insert_into(agent_jwt)
            .values(&new)
            .get_result(conn)?;

        info!(
            id=%record.id,
            registration_id = %record.registration_id,
            jti=%record.jti,
            "Agent JWT record created"
        );

        Ok(record)
    }

    pub fn get_by_jti(
        &self,
        conn: &mut SqliteConnection,
        jti_value: &str,
    ) -> Result<Option<AgentJwt>> {
        let result = agent_jwt
            .filter(jti.eq(jti_value))
            .first::<AgentJwt>(conn)
            .optional()?;

        Ok(result)
    }

    /// Delete a record by name
    pub fn delete_by_jti(&self, conn: &mut SqliteConnection, jti_value: &str) -> Result<usize> {
        match diesel::delete(agent_jwt.filter(jti.eq(jti_value))).execute(conn) {
            Ok(size) => {
                info!(deleted = size, jti = %jti_value, "Agent JWT Record deleted");
                Ok(size)
            }
            Err(error) => {
                warn!(jti = %jti_value, "Failed to delete Agent JWT Record");
                Err(error.into())
            }
        }
    }

    pub fn deactivate_by_jti(
        &self,
        conn: &mut SqliteConnection,
        jti_value: &str,
    ) -> Result<AgentJwt> {
        let mut agent_jwt_record = self.get_by_jti(conn, jti_value)?.unwrap();
        agent_jwt_record.status = AgentJwtStatus::Inactive;

        diesel::update(agent_jwt.filter(jti.eq(jti_value)))
            .set(&agent_jwt_record)
            .returning(AgentJwt::as_returning())
            .get_result::<AgentJwt>(conn)
            .map_err(|error| {
                warn!(jti = jti_value, error = %error, "Failed to deactivate Agent JWT Record");
                error
            })?;

        Ok(agent_jwt_record)
    }

    pub fn activate_by_jti(
        &self,
        conn: &mut SqliteConnection,
        jti_value: &str,
    ) -> Result<AgentJwt> {
        let mut agent_jwt_record = self.get_by_jti(conn, jti_value)?.unwrap();
        agent_jwt_record.status = AgentJwtStatus::Active;

        diesel::update(agent_jwt.filter(jti.eq(jti_value)))
            .set(&agent_jwt_record)
            .returning(AgentJwt::as_returning())
            .get_result::<AgentJwt>(conn)
            .map_err(|error| {
                warn!(jti = jti_value, error = %error, "Failed to deactivate Agent JWT Record");
                error
            })?;

        Ok(agent_jwt_record)
    }

    pub fn expire_by_jti(&self, conn: &mut SqliteConnection, jti_value: &str) -> Result<AgentJwt> {
        let mut agent_jwt_record = self.get_by_jti(conn, jti_value)?.unwrap();
        agent_jwt_record.status = AgentJwtStatus::Expired;

        diesel::update(agent_jwt.filter(jti.eq(jti_value)))
            .set(&agent_jwt_record)
            .returning(AgentJwt::as_returning())
            .get_result::<AgentJwt>(conn)
            .map_err(|error| {
                warn!(jti = jti_value, error = %error, "Failed to deactivate Agent JWT Record");
                error
            })?;

        Ok(agent_jwt_record)
    }
}
