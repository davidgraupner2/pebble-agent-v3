use serde::{Deserialize, Serialize};
use tracing::error;

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct PlatformMessageType {
    #[serde(rename = "$version")]
    pub version: String,
    pub id: String,
    #[serde(rename = "$type")]
    pub message_type: String,
}

pub fn get_message_type(message: &str) -> Option<PlatformMessageType> {
    match serde_json::from_str(message) {
        Ok(message) => message,
        Err(error) => {
            error!(
                "PlatformMessageType: Unable to parse agent type message - {}",
                error
            );

            None
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FunctionCallMessage {
    #[serde(rename(deserialize = "$version"))]
    pub version: String,
    #[serde(rename(deserialize = "$type"))]
    pub message_type: String,
    pub id: String,
    pub name: String,
    scope: String,
    pub token: String,
    pub callback: String,
    pub package_tokens: String,
    api_base: String,
    pub no_cache: bool,
}

pub fn get_function_call_message(message: &str) -> Option<FunctionCallMessage> {
    match serde_json::from_str(message) {
        Ok(message) => message,
        Err(error) => {
            error!(
                "get_function_call_message: Unable to parse agent message - {}",
                error
            );
            None
        }
    }
}
