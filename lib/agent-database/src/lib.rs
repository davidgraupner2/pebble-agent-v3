mod db;
mod errors;
mod models;
pub mod query;
mod repositories;
mod schema;
pub mod traits;
pub mod validators;

use crate::repositories::cache::CacheRepository;
use crate::repositories::connection_stats::ConnectionStatsRepository;
use crate::repositories::encrytion_keys::EncryptionKeyRepository;
use crate::repositories::function_hashes::FunctionHashRepository;
use crate::repositories::registration::RegistrationRepository;
use crate::repositories::registration_challenge::RegistrationChallengeRepository;
use crate::repositories::secrets::SecretRepository;
use crate::repositories::{agent_identity::AgentIdentityRepository, agent_jwt::AgentJwtRepository};

fn source_default() -> Option<String> {
    Some("api".to_string())
}

#[derive(Debug, Clone)]
pub struct RepositoryContainer {
    pub connection_string_repo: ConnectionStringRepository,
    pub events_repo: EventRepository,
    pub properties_repo: PropertyRepository,
    pub function_hash_repo: FunctionHashRepository,
    pub cache_repo: CacheRepository,
    pub connection_stats_repo: ConnectionStatsRepository,
    pub secret_repo: SecretRepository,
    pub encryption_key_repo: EncryptionKeyRepository,
    pub registration_repo: RegistrationRepository,
    pub agent_identity_repo: AgentIdentityRepository,
    pub agent_registration_challenge_repo: RegistrationChallengeRepository,
    pub agent_jwt_repo: AgentJwtRepository,
}

impl RepositoryContainer {
    pub fn initialize() -> RepositoryContainer {
        RepositoryContainer {
            connection_string_repo: ConnectionStringRepository::new(),
            events_repo: EventRepository::new(),
            properties_repo: PropertyRepository::new(),
            function_hash_repo: FunctionHashRepository::new(),
            cache_repo: CacheRepository::new(),
            secret_repo: SecretRepository::new(),
            connection_stats_repo: ConnectionStatsRepository::new(),
            encryption_key_repo: EncryptionKeyRepository::new(),
            registration_repo: RegistrationRepository::new(),
            agent_identity_repo: AgentIdentityRepository::new(),
            agent_registration_challenge_repo: RegistrationChallengeRepository::new(),
            agent_jwt_repo: AgentJwtRepository::new(),
        }
    }
}

//Public re-exports
// pub use db::SqlitePool;
pub use db::{build_database, get_db_connection_pool};
pub use errors::{DatabaseError, Result};
pub use models::{
    AgentIdentity, AgentJwt, AgentJwtStatus, AgentRegistrationChallenge, ApiConnectionString,
    ApiConnectionStringWithEnvironment, ApiEncryptionKey, Cache, CacheWithTags, ConnectionStats,
    ConnectionString, ConnectionStringStatus, CreateCacheRequest, CreateSecretRequest,
    DecryptedSecret, DecryptedSecretWithTags, DecryptionKey, EncryptionKey, Event, EventStatus,
    FunctionHash, NewAgentIdentity, NewAgentJwt, NewAgentRegistrationChallenge, NewCache,
    NewConnectionStats, NewConnectionString, NewEncryptionKey, NewEvent, NewFunctionHash,
    NewRegistration, NewSecret, Property, PropertyRecord, PropertyValue, Registration, Secret,
    SecretTypeInfo, SecretValue, SecretValueOrEncrypted, SecretWithTags, SecureAgentIdentity, Tags,
    TypedProperty, UpdateCache, UpdateCacheRequest, UpdateConnectionStats, UpdateConnectionString,
    UpdateEvent, UpdateFunctionHash, UpdateProperty, UpdateSecret, UpdateSecretRequest,
    UpdatedApiEncryptionKey, get_secret_type_info,
};

pub use repositories::connection_strings::ConnectionStringRepository;
pub use repositories::events::EventRepository;
pub use repositories::properties::PropertyRepository;
pub use traits::{
    RepositoryDynamicQuery, RepositoryGenericInsert, RepositoryGenericUpdate, RepositoryGetSet,
};
