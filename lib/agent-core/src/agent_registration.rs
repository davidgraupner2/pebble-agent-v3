use crate::constants::RuntimeConstants;
use salvo::http::{StatusCode, StatusError};
use salvo::oapi::{self, EndpointOutRegister, ToSchema};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[salvo(schema(example=json!({
	"public_key_b64u": "aTQVr-NTQa_odwywJVUjHx84Sedjx7-yynUAGXNo24c",
	"registration_id": "deb2bc2a4a5b6f8f73d19d532994fd435623862ea5b3fc4783a52ffa63577682"	
})))]
pub struct RegistrationChallengeRequest {
    /// Agent public key used for challenge verification.
    ///
    /// Format:
    /// - Algorithm: Ed25519
    /// - Encoding: base64url without padding
    /// - Content: raw 32-byte public key (not PEM/DER)
    pub public_key_b64u: String,
    /// Stable caller-provided identifier used to correlate registration attempts.
    ///
    /// This value is stored with the challenge and later linked to the created identity.
    pub registration_id: String,
}

impl RegistrationChallengeRequest {
    pub fn new(public_key_b64u: String) -> Self {
        let registration_id = RuntimeConstants::global().id().to_string();
        Self {
            public_key_b64u,
            registration_id,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RegistrationChallengeResponse {
    /// Server-generated identifier for this challenge record.
    ///
    /// The client must send this value to the completion endpoint.
    pub challenge_id: String,
    /// Random nonce the client must sign with the Ed25519 private key.
    ///
    /// Format: base64url without padding.
    pub nonce_b64u: String,
    /// Challenge validity duration in seconds from creation time.
    pub expires_in_sec: u32,
}

impl EndpointOutRegister for RegistrationChallengeResponse {
    fn register(components: &mut oapi::Components, operation: &mut oapi::Operation) {
        operation.responses.insert(
            StatusCode::INTERNAL_SERVER_ERROR.as_str(),
            oapi::Response::new("Internal server error")
                .add_content("application/json", StatusError::to_schema(components)),
        );
        operation.responses.insert(
            StatusCode::NOT_FOUND.as_str(),
            oapi::Response::new("Not found")
                .add_content("application/json", StatusError::to_schema(components)),
        );
        operation.responses.insert(
            StatusCode::BAD_REQUEST.as_str(),
            oapi::Response::new("Bad request")
                .add_content("application/json", StatusError::to_schema(components)),
        );
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CompleteRegistrationRequest {
    /// Challenge identifier returned by `RegistrationChallengeResponse`.
    pub challenge_id: String,
    /// Same Ed25519 public key used to create the challenge.
    ///
    /// Format:
    /// - base64url without padding
    /// - raw 32-byte public key
    pub public_key_b64u: String,
    /// Ed25519 signature over the decoded nonce bytes.
    ///
    /// Format:
    /// - base64url without padding
    /// - raw 64-byte Ed25519 signature
    pub signature_b64u: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CompleteRegistrationResponse {
    /// Server-side unique identity for the registered agent.
    pub registration_id: String,
    /// JWT access token for authenticated API calls.
    pub access_token: String,
    /// Access token lifetime in seconds.
    pub expires_in_sec: i32,
}

impl EndpointOutRegister for CompleteRegistrationResponse {
    fn register(components: &mut oapi::Components, operation: &mut oapi::Operation) {
        operation.responses.insert(
            StatusCode::INTERNAL_SERVER_ERROR.as_str(),
            oapi::Response::new("Internal server error")
                .add_content("application/json", StatusError::to_schema(components)),
        );
        operation.responses.insert(
            StatusCode::NOT_FOUND.as_str(),
            oapi::Response::new("Not found")
                .add_content("application/json", StatusError::to_schema(components)),
        );
        operation.responses.insert(
            StatusCode::BAD_REQUEST.as_str(),
            oapi::Response::new("Bad request")
                .add_content("application/json", StatusError::to_schema(components)),
        );
    }
}
