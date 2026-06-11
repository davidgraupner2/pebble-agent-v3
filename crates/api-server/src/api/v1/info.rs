use crate::api::extensions::DepotExt;
use crate::error::Result;
use agent_core::prelude::*;
use agent_database::{AgentIdentity, SecureAgentIdentity};
use salvo::prelude::*;
use serde::Serialize;
#[derive(Serialize, ToSchema)]
/// Public service metadata returned by the V1 info endpoint.
///
/// This payload is intended for API clients, health dashboards, and operators who need
/// lightweight runtime context without querying internal subsystems.
struct V1Info {
    /// Public API contract version exposed by this endpoint (for example, `V1`).
    api_version: String,
    /// Compiled application version of the running API server binary.
    binary_version: String,
    /// Stable identifier of this API server instance.
    id: String,
    /// Current server status string.
    ///
    /// `Ok` indicates the server is reachable and responded successfully.
    status: String,
    /// Number of agent identities currently registered in the backing database.
    registration_total: i64,
    /// Actual records of agent identities currently registered in the backing database.
    registrations: Vec<SecureAgentIdentity>,
}

/// Returns high-level API server metadata.
///
/// Use this endpoint for quick service introspection and startup compatibility checks.
/// The response includes protocol version, running binary version, instance identifier,
/// a human-readable status, and the total number of registered agents.
#[endpoint(security(("bearer_token"=[])), tags("Information"), status_codes(200, 401, 500))]
async fn info(depot: &mut Depot) -> Result<Json<V1Info>> {
    let properties = RuntimeConstants::global();
    let agent_identity_repo = depot.repositories()?.agent_identity_repo;

    let mut db_connection = depot.db_conn()?;
    let agent_count = agent_identity_repo.get_count(&mut db_connection)?;
    let agent_identities = agent_identity_repo.get_all(&mut db_connection)?;

    let version = properties.version();

    let v1_info = V1Info {
        api_version: "V1".to_string(),
        binary_version: version.to_string(),
        id: properties.api_id().to_string(),
        status: "Ok".to_string(),
        registration_total: agent_count,
        registrations: agent_identities,
    };

    Ok(Json(v1_info))
}

/// Build the V1 information route.
///
/// Mounted endpoint:
/// - `GET /info`
pub fn info_router() -> Router {
    Router::with_path("info").get(info)
}
