use crate::actors::config::arguments::ConfigManagerArguments;
use crate::actors::config::config::AgentConfig;
use crate::actors::config::messages::ConfigManagerMessage;
use crate::actors::config::messages::ConfigUpdate;
use crate::actors::config::state::ConfigManagerState;
use agent_core::prelude::*;
use ractor::Actor;
use ractor::ActorProcessingErr;
use ractor::ActorRef;
use tracing::info;

#[derive(Debug)]
pub struct ConfigManager;

impl Actor for ConfigManager {
    type State = ConfigManagerState;
    type Msg = ConfigManagerMessage;
    type Arguments = ConfigManagerArguments;

    // Invoked when the config is being started
    // Panics in pre_start do not invoke the supervision strategy and the actor won’t be started
    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        _arguments: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        let config_file = RuntimeConstants::global()
            .folders()
            .supplementary_files()
            .join("agent.config");

        // Load config synchronously during actor startup
        let config: AgentConfig = confy::load_path(&config_file)?;
        Ok(ConfigManagerState {
            path: config_file,
            config,
        })
    }

    async fn post_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        _state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        info!("Agent Config has started");

        Ok(())
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            ConfigManagerMessage::GetConfig(reply) => {
                let _ = reply.send(state.config.clone());
            }
            ConfigManagerMessage::UpdateConfig(update) => {
                match update {
                    ConfigUpdate::UpdatePingInterval(new_ping_interval) => {
                        state.config.connection.ping_interval = new_ping_interval
                    }
                    ConfigUpdate::ResetToDefault => state.config = AgentConfig::default(),
                }

                // Flush to disk safely without blocking the async executor
                let path = state.path.clone();
                let config = state.config.clone();
                tokio::task::spawn_blocking(move || confy::store_path(&path, &config)).await??;
            }
        }

        Ok(())
    }
}
