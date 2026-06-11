use crate::error::{ApiError, Result};
use crate::properties::{DEFAULT_PROPERTY_API_JWT_EXPIRY_MINUTES, PROPERTY_API_JWT_EXPIRY_MINUTES};
use crate::{
    api::extensions::DepotExt,
    properties::{
        DEFAULT_PROPERTY_API_AGENT_REGISTRATION_EXPIRY_SECS,
        PROPERTY_API_AGENT_REGISTRATION_EXPIRY_SECS,
    },
};
use agent_core::prelude::{
    generate_agent_jwt, CompleteRegistrationRequest, CompleteRegistrationResponse,
    RegistrationChallengeRequest, RegistrationChallengeResponse, RegistrationClaims,
    RuntimeConstants,
};
use agent_database::{NewAgentIdentity, NewAgentJwt, NewAgentRegistrationChallenge};
use base64ct::{Base64UrlUnpadded, Encoding};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use salvo::{oapi::extract::JsonBody, prelude::*};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Start the agent registration proof-of-possession flow.
///
/// Why this endpoint exists:
/// - The API server must verify that the caller controls a private key before issuing a JWT.
/// - A caller cannot prove key ownership by sending only a public key.
///
/// How it works:
/// - The agent sends `registration_id` and `public_key_b64u` (Ed25519 public key, base64url without padding).
/// - The server stores a short-lived challenge record and returns:
///   - `challenge_id` (lookup key for completion)
///   - `nonce_b64u` (random bytes to be signed by the matching private key)
///   - `expires_in_sec` (challenge validity window)
///
/// Security notes:
/// - This endpoint does not issue credentials.
/// - JWT issuance happens only after the signature is verified by `complete_registration_challenge`.
#[endpoint(tags("Register Agent"), status_codes(200, 400, 500), request_body=RegistrationChallengeRequest)]
async fn registration_challenge(
    depot: &mut Depot,
    challenge_request: JsonBody<RegistrationChallengeRequest>,
) -> Result<Json<RegistrationChallengeResponse>> {
    let registration_id = challenge_request.registration_id.clone();
    let api_id = RuntimeConstants::global().api_id();
    let registration_repo = depot.repositories()?.agent_registration_challenge_repo;

    debug!(registration_id=%registration_id,"Received Registration Request");

    // Get configured parameters we need for the response
    let config = depot.config()?;
    let registration_expiry_in_secs = config.get_int(
        PROPERTY_API_AGENT_REGISTRATION_EXPIRY_SECS,
        DEFAULT_PROPERTY_API_AGENT_REGISTRATION_EXPIRY_SECS,
        api_id.to_string(),
    );

    // Create a challenge response with a nonce we will send back to the requester
    let challenge = NewAgentRegistrationChallenge::new(
        challenge_request.public_key_b64u.clone(),
        registration_id.clone(),
    )
    .map_err(|e| {
        error!(errorMsg=%e, registration_id=%registration_id, "Failed to create registration challenge");
        ApiError::ServerError("Failed to create registration challenge".to_string())
    })?;

    let mut db_connection = depot.db_conn()?;
    let challenge_record = registration_repo.create(&mut db_connection, challenge).map_err(|e| {
        error!(errorMsg=%e, agent_id=%registration_id, "Failed to store registration challenge");
        ApiError::DataAccessError("Failed to store registration challenge".to_string())
    })?;

    debug!(
        registration_id=%registration_id,
        challenge_id=%challenge_record.challenge_id,
        expiry_secs=%registration_expiry_in_secs,
        "Registration challenge stored"
    );

    let challenge_response = RegistrationChallengeResponse {
        challenge_id: challenge_record.challenge_id,
        nonce_b64u: challenge_record.nonce_b64u,
        expires_in_sec: registration_expiry_in_secs as u32,
    };

    Ok(Json(challenge_response))
}

/// Complete the registration challenge and issue an API JWT on success.
///
/// Why this endpoint exists:
/// - It proves the caller owns the private key for the submitted public key.
/// - Only after this proof should the server create/fetch an identity and mint an access token.
///
/// How it works:
/// - The agent submits `challenge_id`, `public_key_b64u`, and `signature_b64u`.
/// - The server loads the stored challenge and validates, in order:
///   1. challenge exists
///   2. challenge is not expired
///   3. submitted public key fingerprint matches challenge fingerprint
///   4. Ed25519 signature over `nonce_b64u` is valid
/// - If all checks pass, the server creates or reuses the agent identity and returns a JWT.
///
/// Security notes:
/// - The nonce is signed, not encrypted.
/// - Signature verification uses Ed25519 and rejects malformed encodings and wrong key sizes.
#[endpoint(tags("Register Agent"), status_codes(200, 400, 404, 500))]
pub async fn complete_registration_challenge(
    depot: &mut Depot,
    complete_challenge_request: JsonBody<CompleteRegistrationRequest>,
) -> Result<Json<CompleteRegistrationResponse>> {
    debug!(challenge_id = %complete_challenge_request.challenge_id,
        "Received registeration challenge response"
    );

    let challenge_id = &complete_challenge_request.challenge_id;
    let api_id = RuntimeConstants::global().api_id();

    // Extract the database connection and database repositories from depot
    let mut db_connection = depot.db_conn()?;
    let registration_repo = depot.repositories()?.agent_registration_challenge_repo;
    let identification_repo = depot.repositories()?.agent_identity_repo;
    let agent_jwt_repo = depot.repositories()?.agent_jwt_repo;

    // Get the configured timeout for the registration record
    let config = depot.config()?;
    let registration_expiry_in_secs = config.get_int(
        PROPERTY_API_AGENT_REGISTRATION_EXPIRY_SECS,
        DEFAULT_PROPERTY_API_AGENT_REGISTRATION_EXPIRY_SECS,
        api_id.to_string(),
    );

    // Retrieve the matching challenge record from the database
    let challenge_record = registration_repo.get_by_challenge_id(&mut db_connection, challenge_id).map_err(|e| {
        error!(errorMsg=%e, challenge_id=%challenge_id, "Error retrieving Registration Challenge record");
        ApiError::DataAccessError("Error retrieving Registration Challenge record".to_string())
    })?;

    if challenge_record.is_none() {
        warn!(challenge_id=%challenge_id,
            "Registation Challenge record not found",
        );
        return Err(ApiError::NotFoundError(
            format!(
                "Registration challenge with id: '{}' not found",
                challenge_id
            )
            .to_string(),
        ));
    }

    let challenge = challenge_record.unwrap();

    // Check that the registration challenge hasn't expired
    let now = chrono::Utc::now().naive_utc();
    let elapsed = now
        .signed_duration_since(challenge.created_at)
        .num_seconds();

    if elapsed > registration_expiry_in_secs as i64 {
        warn!(challenge_id=%challenge_id,"Registration challenge has expired");
        return Err(ApiError::BadRequest(format!(
            "Registration Challenge with id: '{}' has expired!",
            &challenge_id
        )));
    }

    // Verify public key matches
    let fingerprint = compute_pubkey_fingerprint(&complete_challenge_request.public_key_b64u)?;
    if fingerprint != challenge.pubkey_fingerprint_b64u {
        error!(challenge_id=%challenge_id,"Registration challenge has a public key mismatch");
        return Err(ApiError::BadRequest(
            format!(
                "Registation challenge with id '{}' has a public key mismatch",
                &challenge_id
            )
            .to_string(),
        ));
    }

    // Verify signature
    verify_challenge_signature(
        &complete_challenge_request.public_key_b64u,
        &challenge.nonce_b64u,
        &complete_challenge_request.signature_b64u,
    )?;

    // Create or fetch agent identity
    let agent_identity = match identification_repo
        .get_by_fingerprint(&mut db_connection, &fingerprint)
    {
        Ok(Some(existing)) => existing,
        Ok(None) => {
            let new_agent = NewAgentIdentity::new(
                fingerprint.clone(),
                complete_challenge_request.public_key_b64u.clone(),
                challenge.registration_id.clone(),
            );
            identification_repo.create(&mut db_connection, new_agent)
                .map_err(|e| {
                    error!(errorMsg=%e, challenge_id=%challenge_id,"Failed to create identity record");
                    ApiError::DataAccessError(e.to_string())
                })?
        }
        Err(e) => {
            error!(errorMsg=%e,challenge_id=%challenge_id,agent_id=%&challenge.registration_id,"Failed to retrieve identity record");
            return Err(ApiError::DataAccessError(e.to_string()));
        }
    };

    // Get the JWT expiry setting from the database
    let jwt_expiry_in_mins = config.get_int(
        PROPERTY_API_JWT_EXPIRY_MINUTES,
        DEFAULT_PROPERTY_API_JWT_EXPIRY_MINUTES,
        api_id.to_string(),
    );

    //  calculate now in secs
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Calculates when the JWT expires in Secs
    let jwt_expires_in_secs = if jwt_expiry_in_mins <= 0 {
        None
    } else {
        Some(now + (jwt_expiry_in_mins as u64) * 60)
    };

    // Issue JWT for the registered agent
    let jti = Uuid::new_v4().to_string();
    let claims = RegistrationClaims::new(
        agent_identity.registration_id.clone(),
        jti.clone(),
        Some(fingerprint.clone()),
        jwt_expires_in_secs,
        now,
    );
    let jwt = generate_agent_jwt(&claims).map_err(|e| {
        error!(errorMsg=%e,challenge_id=%challenge_id,agent_id=%&challenge.registration_id,"JWT generation failed");
        ApiError::ServerError("Failed to issue registration access token".to_string())
    })?;

    // Deactivate all previous jti records
    let count_jti_deactivated = agent_jwt_repo
        .deactivate_by_registration_id(&mut db_connection, &agent_identity.registration_id)?;
    debug!(count=%count_jti_deactivated,agent_id=%challenge.registration_id,"Deactivated JTI records");

    // Save the new JTI So that it can be invalidated when needed
    let _agent_jwt_record = agent_jwt_repo.create(
        &mut db_connection,
        NewAgentJwt {
            registration_id: agent_identity.registration_id.clone(),
            jti: jti,
            status: agent_database::AgentJwtStatus::Active,
        },
    )?;

    info!(
        registration_id = %&agent_identity.registration_id   ,challenge_id=%challenge_id,agent_id=%&challenge.registration_id,
        "Identification complete via registration challenge response"
    );

    // Delete the challenge record as we no longer need it.
    let _ = registration_repo.delete_by_challenge_id(&mut db_connection, challenge_id);

    Ok(Json(CompleteRegistrationResponse {
        registration_id: agent_identity.registration_id,
        access_token: jwt,
        expires_in_sec: jwt_expiry_in_mins * 60,
    }))
}

/// Build all V1 registration routes.
///
/// Mounted endpoints:
/// - `POST /registration/challenge`
/// - `POST /registration/complete`
pub fn registration_router() -> Router {
    Router::new()
        .push(Router::with_path("registration/challenge").post(registration_challenge))
        .push(Router::with_path("registration/complete").post(complete_registration_challenge))
}

/// Compute a deterministic SHA-256 fingerprint from a base64url-encoded public key.
///
/// The resulting digest is returned as base64url without padding.
fn compute_pubkey_fingerprint(public_key_b64u: &str) -> Result<String> {
    let pk_bytes = Base64UrlUnpadded::decode_vec(public_key_b64u)
        .map_err(|_| ApiError::BadRequest("Invalid public key encoding".to_string()))?;

    let digest = Sha256::digest(&pk_bytes);
    Ok(Base64UrlUnpadded::encode_string(&digest))
}

/// Verify an Ed25519 signature for the registration challenge nonce.
///
/// Inputs are expected as base64url (no padding). Any decode, size, key, or
/// signature validation failure maps to `ApiError::BadRequest`.
fn verify_challenge_signature(
    public_key_b64u: &str,
    nonce_b64u: &str,
    signature_b64u: &str,
) -> Result<()> {
    let pk_bytes = Base64UrlUnpadded::decode_vec(public_key_b64u)
        .map_err(|_| ApiError::BadRequest("Invalid public key".to_string()))?;

    let nonce = Base64UrlUnpadded::decode_vec(nonce_b64u)
        .map_err(|_| ApiError::BadRequest("Invalid nonce".to_string()))?;

    let sig_bytes = Base64UrlUnpadded::decode_vec(signature_b64u)
        .map_err(|_| ApiError::BadRequest("Invalid signature".to_string()))?;

    let vk = VerifyingKey::from_bytes(
        &pk_bytes
            .as_slice()
            .try_into()
            .map_err(|_| ApiError::BadRequest("Wrong public key size".to_string()))?,
    )
    .map_err(|_| ApiError::BadRequest("Bad public key".to_string()))?;

    let sig = Signature::from_slice(&sig_bytes)
        .map_err(|_| ApiError::BadRequest("Bad signature bytes".to_string()))?;

    vk.verify(&nonce, &sig).map_err(|_| {
        warn!("Signature verification failed");
        ApiError::BadRequest("Signature verification failed".to_string())
    })?;

    Ok(())
}
