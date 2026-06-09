use std::path::PathBuf;

use crate::actors::config::config::AgentConfig;

pub struct ConfigManagerState {
    pub path: PathBuf,
    pub config: AgentConfig,
}
