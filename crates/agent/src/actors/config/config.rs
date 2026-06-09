use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct ProxySettings {
    pub server: String,
    pub port: u16,
    pub username: String,
    pub password: String,
}

impl Default for ProxySettings {
    fn default() -> Self {
        Self {
            server: "".to_string(),
            port: 0,
            username: "".to_string(),
            password: "".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct ApiServerSettings {
    pub standalone: bool,
    pub host: String,
    pub port: u16,
}

impl Default for ApiServerSettings {
    fn default() -> Self {
        Self {
            standalone: true,
            host: "127.0.0.1".to_string(),
            port: 8174,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct LoggingSettings {
    pub format: String,
    pub output: String,
    pub level: String,
}

impl Default for LoggingSettings {
    fn default() -> Self {
        Self {
            format: "pretty".to_string(),
            output: "file".to_string(),
            level: "error".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct ConnectionSettings {
    pub connection_strings: Vec<String>,
    pub ping_interval: u16,
    pub timeout: u16,
    pub retry_interval: u16,
    pub pong_response_interval: u16,
}

impl Default for ConnectionSettings {
    fn default() -> Self {
        Self {
            timeout: 10,
            retry_interval: 30,
            pong_response_interval: 30,
            connection_strings: vec![],
            ping_interval: 30,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct AgentConfig {
    pub api_server: ApiServerSettings,
    pub proxy: ProxySettings,
    pub logging: LoggingSettings,
    pub connection: ConnectionSettings,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            proxy: ProxySettings::default(),
            api_server: ApiServerSettings::default(),
            logging: LoggingSettings::default(),
            connection: ConnectionSettings::default(),
        }
    }
}
