use crate::registration::keys::AgentRegistrationKeyPair;
use agent_core::prelude::*;
use anyhow::{anyhow, Result};
use reqwest::Client;
use std::path::PathBuf;

// Generates a Registration Challenge request
// - With a public key and the agent id
// - This is sent to the API Server to start an agent registration
pub fn generate_agent_registration_request(public_key: &str) -> RegistrationChallengeRequest {
    RegistrationChallengeRequest::new(public_key.to_string())
}

pub async fn get_api_jwt(
    challenge_url: &str,
    challenge_complete_url: &str,
) -> Result<CompleteRegistrationResponse> {
    let keypair_file = RuntimeConstants::global().agent_registration_key_file();
    let keypair: AgentRegistrationKeyPair = get_agent_keypair(&keypair_file)?;

    let registration_challenge =
        get_agent_registration_challenge(challenge_url, &keypair.public_key_b64u).await?;

    let signed_registration_nonce = keypair.sign_challenge(&registration_challenge.nonce_b64u)?;

    let challenge_response = complete_agent_registration(
        challenge_complete_url,
        registration_challenge,
        &keypair.public_key_b64u,
        &signed_registration_nonce,
    )
    .await?;

    Ok(challenge_response)
}

// Sends the agent registration request to the API Server
// The API Server returns a response which contains:
// - id (identifing he challenge)
// - nonce the agent must sign with the Ed25519 private key
// - expiry time
pub async fn get_agent_registration_challenge(
    url: &str,
    public_key: &str,
) -> Result<RegistrationChallengeResponse> {
    let registration_challenge = generate_agent_registration_request(public_key);

    let client = Client::new();
    let challenge_resp = match client.post(url).json(&registration_challenge).send().await {
        Ok(response) => match response.json::<RegistrationChallengeResponse>().await {
            Ok(data) => data,
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "Failed to parse agent registration challenge request: {}",
                    e
                ))
            }
        },
        Err(e) => {
            return Err(anyhow::anyhow!(
                "Client error during agent registration challenge request: {}",
                e
            ))
        }
    };

    return Ok(challenge_resp);
    // if challenge_resp {
    //     return Ok(challenge_resp.data.unwrap());
    // } else {
    //     return Err(anyhow::anyhow!(challenge_resp.error.unwrap()));
    // }
}

// Completes the agent registration with the API Server
// - if successful, response is returned with
// - agent uuid (unique identifier for the agent in the api database)
// - access_token (jwt to access the api server)
// - expiry time for the jwt
pub async fn complete_agent_registration(
    url: &str,
    agent_registration_challenge: RegistrationChallengeResponse,
    public_key_b64u: &str,
    signature_b64u: &str,
) -> Result<CompleteRegistrationResponse> {
    let client = Client::new();

    let challenge_complete = match client
        .post(url)
        .json(&serde_json::json!({
            "challenge_id": agent_registration_challenge.challenge_id,
            "public_key_b64u": public_key_b64u,
            "signature_b64u": signature_b64u,
        }))
        .send()
        .await
    {
        Ok(response) => match response.json::<CompleteRegistrationResponse>().await {
            Ok(data) => data,
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "Failed to parse agent registration challenge response: {}",
                    e
                ))
            }
        },
        Err(e) => {
            return Err(anyhow::anyhow!(
                "Client error during challenge response: {}",
                e
            ))
        }
    };

    return Ok(challenge_complete);
    // if challenge_complete.ok {
    //     return Ok(challenge_complete.data.unwrap());
    // } else {
    //     return Err(anyhow::anyhow!(challenge_complete.error.unwrap()));
    // }
}

// First time its called:
// - Generates a persistent public/private key combination for the agent
// - After that it returns the same persistent key combination retrieved for a key file
fn get_agent_keypair(keypair_file: &PathBuf) -> anyhow::Result<AgentRegistrationKeyPair> {
    let keypair: AgentRegistrationKeyPair = match std::fs::read_to_string(keypair_file) {
        Ok(content) => match serde_json::from_str(&content) {
            Ok(keypair) => {
                println!("Found Key Pair: {:#?}", keypair);
                keypair
            }
            Err(_) => {
                let new_keypair = AgentRegistrationKeyPair::generate();
                std::fs::write(keypair_file, serde_json::to_string(&new_keypair)?)?;
                println!("Generated new Key Pair: {:#?}", new_keypair);
                new_keypair
            }
        },
        Err(_) => {
            let new_keypair = AgentRegistrationKeyPair::generate();
            std::fs::write(keypair_file, serde_json::to_string(&new_keypair)?)?;
            println!("Generated new Key Pair: {:#?}", new_keypair);
            new_keypair
        }
    };
    Ok(keypair)
}
