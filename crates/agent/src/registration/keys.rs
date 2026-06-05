use base64ct::{Base64UrlUnpadded, Encoding};
use ed25519_dalek::{Signer, SigningKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};

/// Persisted agent identity — store this on disk (e.g. agent.key.json).
/// Never transmit `private_key_b64u`.
///
/// Key format notes:
/// - Algorithm: Ed25519 (EdDSA), not RSA.
/// - `public_key_b64u`: base64url (no padding) of the raw 32-byte Ed25519 public key.
/// - `private_key_b64u`: base64url (no padding) of the raw 32-byte Ed25519 secret key.
///
/// These are raw key bytes encoded as base64url, not PEM/DER/PKCS#8 containers.
#[derive(Debug, Serialize, Deserialize)]
pub struct AgentRegistrationKeyPair {
    pub public_key_b64u: String,
    pub private_key_b64u: String,
}

impl AgentRegistrationKeyPair {
    /// Generate a new Ed25519 keypair.
    pub fn generate() -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        let public_key_b64u =
            Base64UrlUnpadded::encode_string(signing_key.verifying_key().as_bytes());
        let private_key_b64u = Base64UrlUnpadded::encode_string(signing_key.as_bytes());
        Self {
            public_key_b64u,
            private_key_b64u,
        }
    }

    /// Load from bytes (e.g. read from disk).
    pub fn from_private_key_b64u(private_key_b64u: &str) -> anyhow::Result<Self> {
        let raw = Base64UrlUnpadded::decode_vec(private_key_b64u)
            .map_err(|_| anyhow::anyhow!("Invalid base64url private key"))?;
        let bytes: [u8; 32] = raw
            .try_into()
            .map_err(|_| anyhow::anyhow!("Wrong private key length"))?;
        let signing_key = SigningKey::from_bytes(&bytes);
        let public_key_b64u =
            Base64UrlUnpadded::encode_string(signing_key.verifying_key().as_bytes());
        Ok(Self {
            public_key_b64u,
            private_key_b64u: private_key_b64u.to_string(),
        })
    }

    /// Sign the nonce received from `POST /register/challenge`.
    /// Pass `nonce_b64u` directly from the server response.
    pub fn sign_challenge(&self, nonce_b64u: &str) -> anyhow::Result<String> {
        let raw_key = Base64UrlUnpadded::decode_vec(&self.private_key_b64u)
            .map_err(|_| anyhow::anyhow!("Invalid private key"))?;
        let bytes: [u8; 32] = raw_key
            .try_into()
            .map_err(|_| anyhow::anyhow!("Wrong private key length"))?;
        let signing_key = SigningKey::from_bytes(&bytes);

        let nonce = Base64UrlUnpadded::decode_vec(nonce_b64u)
            .map_err(|_| anyhow::anyhow!("Invalid nonce"))?;

        let signature = signing_key.sign(&nonce);
        Ok(Base64UrlUnpadded::encode_string(
            signature.to_bytes().as_ref(),
        ))
    }
}
