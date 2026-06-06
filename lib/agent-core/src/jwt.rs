use crate::constants::RuntimeConstants;
use jsonwebtoken::{encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use machineid_rs::{Encryption, HWIDComponent, IdBuilder};
use salvo::Depot;
use salvo_jwt_auth::JwtAuth;
use salvo_jwt_auth::{ConstDecoder, HeaderFinder};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use std::env::home_dir;

/// Claims issued to a registered agent.
///
/// `sub`     — the agent's stable UUID (prevents identity spoofing).
/// `cnf_jkt` — SHA-256 fingerprint of the agent's Ed25519 public key,
///             stored for future DPoP / proof-of-possession enforcement.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RegistrationClaims {
    pub iss: String,
    pub sub: String,
    pub aud: String,
    pub jti: String,
    pub iat: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exp: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cnf_jkt: Option<String>,
}

impl RegistrationClaims {
    pub fn new(
        agent_uuid: String,
        jti: String,
        pubkey_fingerprint: Option<String>,
        expires_in_sec: Option<u64>,
        now: u64,
    ) -> Self {
        let api_id = RuntimeConstants::global().api_id().to_string();
        Self {
            iss: api_id.clone(),
            sub: agent_uuid,
            aud: api_id,
            jti,
            iat: now,
            exp: expires_in_sec,
            cnf_jkt: pubkey_fingerprint,
        }
    }
}

/// Signs an [`AgentClaims`] token using the same HWID-derived key as
/// [`generate_secure_jwt_with_hwid`], ensuring tokens are only valid on
/// the machine that issued them.
pub fn generate_agent_jwt(
    claims: &RegistrationClaims,
) -> Result<String, jsonwebtoken::errors::Error> {
    let secure_secret = jwt_secret();
    let header = Header::new(Algorithm::HS256);
    let token = {
        let exposed_key = secure_secret.expose_secret();
        let encoding_key = EncodingKey::from_secret(exposed_key.as_bytes());
        encode(&header, claims, &encoding_key)?
    };

    Ok(token)
}

pub fn jwt_secret() -> SecretString {
    let current_dir = home_dir()
        .unwrap()
        .into_os_string()
        .into_string()
        .unwrap()
        .replace("/", "")
        .replace("\\", "");
    let current_dir_static: &'static str = Box::leak(current_dir.into_boxed_str());

    let mut hardware_id_builder = IdBuilder::new(Encryption::SHA256);
    hardware_id_builder
        .add_component(HWIDComponent::SystemID)
        .add_component(HWIDComponent::CPUID)
        .add_component(HWIDComponent::OSName)
        .add_component(HWIDComponent::FileToken(&current_dir_static))
        .add_component(HWIDComponent::FileToken("b7a6d89201f3e4c5b6a7d8c901e2f3a4"));

    let raw_hwid = hardware_id_builder.build("jwt_secret").unwrap_or_default();
    SecretString::new(raw_hwid.into())
}

pub fn auth_jwt_middleware() -> JwtAuth<RegistrationClaims, ConstDecoder> {
    let jwt_secret = jwt_secret();

    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;
    validation.validate_aud = true;
    validation.set_required_spec_claims(&[""; 0]);
    validation.set_audience(&[RuntimeConstants::global().api_id()]);

    let decoding_key = DecodingKey::from_secret(jwt_secret.expose_secret().as_bytes());
    let decoder = ConstDecoder::with_validation(decoding_key, validation);

    // Construct the Salvo JwtAuth middleware layer
    JwtAuth::new(decoder).finders(vec![Box::new(HeaderFinder::new())]) // Extract from "Authorization: Bearer <token>"
}
